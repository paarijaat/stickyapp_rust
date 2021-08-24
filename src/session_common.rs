use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum SessionRequestCommand {
    SessionCommand(String),
    SessionStop,
}

#[derive(Debug, Clone)]
pub enum SessionResponseStatus {
    SessionOk,
    SessionExit,
}

#[derive(Deserialize,Debug)]
pub struct SessionRequestMessage {
    pub action: String,
    pub value: f64,
}

#[derive(Serialize)]
pub struct SessionResponseMessage {
    pub status: bool,
    pub status_message: String,
    pub value: f64,
}
impl Default for SessionResponseMessage {
    fn default() -> Self { 
        Self {
            status: false,
            status_message: String::from(""),
            value: 0.0
        }
    }
}

pub fn session_status_to_string(status: &SessionResponseStatus) -> String {
    match status {
        SessionResponseStatus::SessionOk => "SessionOk".into(),
        SessionResponseStatus::SessionExit => "SessionExit".into(),
    }
}


pub type SenderSessionResponseChannel = oneshot::Sender<(SessionResponseStatus, String)>;
pub type ReceiverSessionResponseChannel = oneshot::Receiver<(SessionResponseStatus, String)>;
pub type SenderSessionRequestChannel = Sender<(SessionRequestCommand, SenderSessionResponseChannel)>;
pub type ReceiverSessionRequestChannel = Receiver<(SessionRequestCommand, SenderSessionResponseChannel)>;

pub async fn send_command(
    sessionid: &str, 
    request_channel_tx: SenderSessionRequestChannel,
    cmd: SessionRequestCommand,
) -> Result<(SessionResponseStatus, String), String> {
    tracing::debug!("[{}] Sending SessionCommand.", sessionid);

    // create a one time command response channel
    let (resp_tx, resp_rx) = oneshot::channel::<(SessionResponseStatus, String)>();

    // send command on the main communication channel for the session
    let command_sent = request_channel_tx.send((cmd,resp_tx)).await;
    match command_sent {
        Ok(()) => {
            let session_result = resp_rx.await;
            match session_result {
                Ok((status,response)) => {
                    tracing::debug!("[{}] Received response from session. Status: {}", sessionid, session_status_to_string(&status));
                    Ok((status,response))
                }
                Err(e) => {
                    let err_msg = format!("[{}] Failed to receive response from session. {}", sessionid, e.to_string());
                    tracing::warn!("{}", err_msg);
                    Err(err_msg)
                }
            }
        }
        Err(_) => {
            let err_msg = format!("[{}] Failed to send command to session.", sessionid);
            tracing::warn!("{}", err_msg);
            Err(err_msg)
        }
    }
}

pub fn send_response(
    sessionid: &String, 
    response_status: SessionResponseStatus, 
    response_message: SessionResponseMessage, 
    response_tx: SenderSessionResponseChannel
) -> bool {
    if let Err(_) = response_tx.send((response_status, serde_json::to_string(&response_message).unwrap())) {
        tracing::warn!("[{}] Error sending Response for SessionCommand", sessionid);
        return false;
    }
    true
}

pub async fn wait_for_init(sessionid: &str, init_success_rx: ReceiverSessionResponseChannel) -> Result<String, String> {
    // check if initialization succeeded
    let init_success = init_success_rx.await;
    match init_success {
        Ok((init_status,init_response)) => {
            tracing::debug!("[{}] Received init response from session. Status: {}", sessionid, session_status_to_string(&init_status));
            match init_status {
                SessionResponseStatus::SessionOk => Ok(init_response),
                SessionResponseStatus::SessionExit => {
                    let err_msg = format!("[{}] Init failed for session. {}", sessionid, init_response);
                    tracing::warn!("{}", err_msg);
                    Err(init_response)
                }
            }
        }
        Err(e) => {
            let err_msg = format!("[{}] Init failed for session. {}", sessionid, e.to_string());
            tracing::warn!("{}", err_msg);
            Err(err_msg)
        }
    }
}

pub fn new_session_id(encrypted: bool) -> String {
    let mut prefix = String::from("open");
    if encrypted {
        prefix = String::from("enc");
    }
    prefix + Uuid::new_v4().to_simple().encode_lower(&mut Uuid::encode_buffer()).to_string().as_str()
}