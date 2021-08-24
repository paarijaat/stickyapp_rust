use axum::{
    prelude::*, 
    extract::{Extension, Path},
    response::{IntoResponse}, 
    http::{StatusCode,header::HeaderName,HeaderValue,Response},
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tower::{BoxError, ServiceBuilder};
use tower_http::{
    add_extension::AddExtensionLayer, trace::TraceLayer,
};
use chrono::prelude::*;
use tokio::signal::unix::{signal, SignalKind};

mod session;
mod session_encrypted;
mod session_common;
use session_common::*;
mod utils;

// List sessions response message
#[derive(Serialize)]
struct ListSessionsResponse {
    message: String,
    sessionids: Vec<String>,
}
// Session action request/response messages
#[derive(Deserialize)]
struct SessionRequest {
    message: String,
}
#[derive(Serialize)]
struct SessionResponse {
    status: bool,
    message: String,
    sessionid: String,
}

// Shared state storing communication channel to sessions
type SharedState = Arc<RwLock<State>>;
struct State {
    db: HashMap<String, session_common::SenderSessionRequestChannel>,
    localip: String,
    shutdown_tx: tokio::sync::mpsc::Sender<()>,
}

// basic handler that responds with a static string
async fn root(Extension(state): Extension<SharedState>,) -> String {
    let msg_str = format!("ok, localip {}, {}", &state.read().unwrap().localip, Local::now().to_rfc2822());
    tracing::info!("Request for / , returning: {}", msg_str);
    msg_str
}

async fn list_sessions(
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    tracing::info!("list_sessions request received");
    let mut sessionids: Vec<String> = Vec::new();
    {
        let db = &state.read().unwrap().db;
        for (sessionid, _) in db {
            tracing::debug!("[{}] session found", sessionid);
            sessionids.push(sessionid.clone());
        }
    }
    let list_sessions_response = ListSessionsResponse {
        message: String::from("Ok"),
        sessionids
    };
    response::Json(list_sessions_response)
}

#[derive(Deserialize)]
struct SessionRequestQuery {
    encrypted: Option<bool>,
}
impl Default for SessionRequestQuery {
    fn default() -> Self { 
        Self {encrypted: Some(false)}
    }
}
async fn create_session(
    extract::Json(create_request): extract::Json<SessionRequest>,
    Extension(state): Extension<SharedState>,
    session_query: extract::Query<SessionRequestQuery>
) -> impl IntoResponse {
    let need_encrypted_session: bool = match session_query.encrypted {
        Some(e) => e,
        None => false
    };
    let sessionid = new_session_id(need_encrypted_session);
    tracing::debug!("[{}] Trying, Session creation. Encrypted: {}", sessionid, need_encrypted_session);
    let mut localip = String::from("");
    let mut create_response = SessionResponse {
        status: true,
        message: String::from(""),
        sessionid: sessionid.clone(),
    }; 

    // create the main channel for communicating with session
    let (request_channel_tx, request_channel_rx) =
        tokio::sync::mpsc::channel::<(SessionRequestCommand, SenderSessionResponseChannel)>(100);
    
    // create a one-time channel to check if the session started correctly
    let (init_success_tx, init_success_rx) = tokio::sync::oneshot::channel::<(SessionResponseStatus, String)>();

    if need_encrypted_session {
        session_encrypted::spawn(
            sessionid.clone(), 
            create_request.message, 
            request_channel_rx, 
            init_success_tx
        ).await;
    } else {
        session::spawn(
            sessionid.clone(), 
            create_request.message, 
            request_channel_rx, 
            init_success_tx
        ).await;
    }

    let session_spawned = wait_for_init(sessionid.as_str(), init_success_rx).await;
    match session_spawned {
        Ok(init_response) => {
            {
                // Add the main communication channel with the session into shared state
                //state.write().unwrap().db.insert(sessionid.clone(), request_channel_tx);
                let mut shared_state = state.write().unwrap();
                shared_state.db.insert(sessionid.clone(), request_channel_tx);
                localip = shared_state.localip.clone();
            }
            tracing::info!("[{}] Success, Session created at {}. {}", sessionid, localip, init_response);
            create_response.message = init_response;
        }
        Err(error_msg) => {
            let err_msg = format!("[{}] Failure while creating session. {}", sessionid, error_msg);
            tracing::warn!("{}", err_msg);
            create_response.status = false;
            create_response.message = err_msg
        }
    }
    let mut response: Response<Body> = response::Json(create_response).into_response();
    response.headers_mut().insert(
        HeaderName::from_static("x-sessionid"),
        HeaderValue::from_str(sessionid.as_str()).unwrap(),
    );
    response.headers_mut().insert(
        HeaderName::from_static("x-sessionlocation"),
        HeaderValue::from_str(localip.as_str()).unwrap(),
    );
    response
}

async fn session_action(
    Path(sessionid): Path<String>,
    extract::Json(action_request): extract::Json<SessionRequest>,
    Extension(state): Extension<SharedState>,
) -> impl IntoResponse {
    tracing::debug!("[{}] session_action request received", sessionid);

    // Access shared state to extract session info
    let session_info = 
    {
        let db = &state.read().unwrap().db;
        match db.get(&sessionid) {
            Some(session_channel) => Some(session_channel.clone()),
            None => None, 
        }
    };

    // Create an empty response
    let mut action_response = SessionResponse {
        status: true,
        message: String::from(""),
        sessionid: sessionid.clone(),
    };

    match session_info {
        Some(request_channel_tx) => {
            let command_response = session_common::send_command(
                &sessionid, request_channel_tx, 
                session_common::SessionRequestCommand::SessionCommand(action_request.message)
            ).await;
            match command_response {
                Ok((response_status,response_message)) => {
                    action_response.message = response_message;
                    if let session_common::SessionResponseStatus::SessionExit = response_status {
                        state.write().unwrap().db.remove(&sessionid);
                        tracing::info!("[{}] Removing session", sessionid);
                    }
                }
                Err(err_str) => {
                    let err_msg = format!("[{}] Failure executing session command. {}", sessionid, err_str);
                    tracing::warn!("{}", err_msg);
                    action_response.status = false;
                    action_response.message = err_msg;
                }
            }
        }
        None => {
            let err_msg = format!("[{}] Failure. Session not found", sessionid);
            tracing::warn!("{}", err_msg);
            action_response.status = false;
            action_response.message = err_msg;
        }
    }
    response::Json(action_response)
}

async fn shutdown_handler(
    Extension(state): Extension<SharedState>,
) -> &'static str {
    tracing::warn!("Server Shutdown request received");
    {
        let shared_state = state.read().unwrap();
        for (sessionid, request_channel_tx) in &shared_state.db {
            tracing::info!("[{}] sending stop command to session", sessionid);
            tokio::spawn(
                session_common::send_command(
                    "", 
                    request_channel_tx.clone(), 
                    session_common::SessionRequestCommand::SessionStop
                )
            );
        }
        tracing::info!("Signalling server to shutdown");
        let shutdown_tx = shared_state.shutdown_tx.clone();
        tokio::spawn(async move {
            let _ = shutdown_tx.send(()).await;
        });
    }
    "ok"
}


fn handle_error(error: BoxError) -> Result<impl IntoResponse, Infallible> {
    if error.is::<tower::timeout::error::Elapsed>() {
        return Ok((StatusCode::REQUEST_TIMEOUT, Cow::from("request timed out")));
    }

    if error.is::<tower::load_shed::error::Overloaded>() {
        return Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            Cow::from("service is overloaded, try again later"),
        ));
    }

    Ok((
        StatusCode::INTERNAL_SERVER_ERROR,
        Cow::from(format!("Unhandled internal error: {}", error)),
    ))
}


#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        //std::env::set_var("RUST_LOG", "debug");
        std::env::set_var("RUST_LOG", "info,tower_http=info,hyper=info")
    }

    // initialize tracing
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT").unwrap_or("8080".into()).parse::<u16>().unwrap();

    let localip = match utils::get_local_ip_address() {
        Some(ip) => ip,
        None => {
            tracing::error!("Unable to obtain local ip address");
            return;
        }
    };

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    //let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    tracing::info!("Starting at localip {}", localip);
    let shared_state = State {
        db: HashMap::new(),
        localip: localip.clone(),
        shutdown_tx,
    };

    // build our application with a route
    let app =
        // `GET /` goes to `root`
        route("/", get(root))
        .route("/shutdown", get(shutdown_handler))
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/:sid", post(session_action))
        .layer(
            ServiceBuilder::new()
                .load_shed()
                .concurrency_limit(1024)
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .layer(AddExtensionLayer::new(Arc::new(RwLock::new(shared_state))))
                .into_inner(),
        )
        // Handle errors from middleware
        .handle_error(handle_error);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(async {
            let mut signal_sigint = signal(SignalKind::interrupt()).unwrap();
            let mut signal_sigterm = signal(SignalKind::terminate()).unwrap();
            let mut signal_sigquit = signal(SignalKind::quit()).unwrap();
            tokio::select! {
                _ = shutdown_rx.recv() => {}
                _ = signal_sigint.recv() => {tracing::warn!("SIGINT received");}
                _ = signal_sigterm.recv() => {tracing::warn!("SIGTERM received");}
                _ = signal_sigquit.recv() => {tracing::warn!("SIGQUIT received");}
            }
            //shutdown_rx.recv().await;
            tracing::info!("Server will finish in two seconds");
            tokio::time::sleep(Duration::from_millis(1000)).await;
        })
        .await
        .unwrap();
    tracing::info!("Server finished");
}
