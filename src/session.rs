use crate::session_common::*;

pub async fn spawn(
    sessionid: String, init_message: String,
    request_channel_rx: ReceiverSessionRequestChannel,
    init_success_tx: SenderSessionResponseChannel,
) -> () {
    tracing::info!("[{}] Spawning session", sessionid);
    // launch the session loop as a tokio task
    tokio::spawn(session_loop(sessionid.clone(), request_channel_rx, init_message, init_success_tx));
}

async fn session_loop(
    sessionid: String, 
    mut request_channel_rx: ReceiverSessionRequestChannel,
    _init_message: String,
    init_success_tx: SenderSessionResponseChannel,
) -> Result<(), ()> {
    tracing::info!("[{}] Starting session loop", sessionid);

    let mut values: Vec<f64> = Vec::new();

    if !send_response(
        &sessionid,
        SessionResponseStatus::SessionOk,
        SessionResponseMessage{status: true, status_message: String::from("Session Initialized"), ..SessionResponseMessage::default()},
        init_success_tx
    ) {
        return Err(())
    }
    tracing::info!("[{}] Session initialized", sessionid);

    // Init has succeeded. Start main message loop
    while let Some((cmd, resp)) = request_channel_rx.recv().await {
        match cmd {
            SessionRequestCommand::SessionStop => {
                let status_message = format!("[{}] Stopping session", sessionid);
                tracing::info!("{}", status_message);
                if !send_response(
                    &sessionid, 
                    SessionResponseStatus::SessionExit, 
                    SessionResponseMessage{status: true, status_message, ..SessionResponseMessage::default()}, 
                    resp) {
                    tracing::warn!("[{}] Error sending Response for SessionStop", sessionid);
                }
                break;
            }

            SessionRequestCommand::SessionCommand(message) => {
                tracing::debug!("[{}] Received SessionCommand. Message: {}", sessionid, message);
                let request_message: SessionRequestMessage = match serde_json::from_str(message.as_str()) {
                    Ok(m) => m,
                    Err(e) => {
                        let status_message = format!("[{}] Failed to json decode request's message field. {}", sessionid, e.to_string());
                        tracing::warn!("{}", status_message);
                        send_response(
                            &sessionid, 
                            SessionResponseStatus::SessionOk, 
                            SessionResponseMessage{status: false, status_message, ..SessionResponseMessage::default()},
                            resp
                        );
                        continue;
                    }
                };

                tracing::debug!("[{}] Received message: {:?}", sessionid, request_message);


                match request_message.action.as_str() {
                    "encrypt" => {
                        let msg_str = format!("[{}] Encrypt action received. Value {}", sessionid, request_message.value);
                        tracing::debug!("{}", msg_str);

                        values.push(request_message.value);

                        let response_message = SessionResponseMessage {
                            status: true,
                            status_message: msg_str,
                            value: request_message.value,
                        };

                        send_response(&sessionid, SessionResponseStatus::SessionOk, response_message, resp);
                        continue;
                    }
                    "mean" => {
                        let msg_str = format!("[{}] Mean action received", sessionid);
                        tracing::debug!("{}", msg_str);
                        let mut response_message = SessionResponseMessage{status_message: msg_str, ..SessionResponseMessage::default()};
                        
                        if values.len() > 0 {
                            response_message.status = true;
                            let mut sum: f64 = 0.;
                            for value in &values {
                                sum += value;
                            };
                            response_message.value = sum / (values.len() as f64);
                            values.clear();
                        }
                        send_response(&sessionid, SessionResponseStatus::SessionOk, response_message, resp);
                        continue;
                    }
                    "shutdown" => {
                        let err_str = format!("[{}] Shutdown action recevied", sessionid);
                        tracing::info!("{}", err_str);
                        let response_message = SessionResponseMessage{status: true, status_message: err_str, ..SessionResponseMessage::default()};
                        send_response(&sessionid, SessionResponseStatus::SessionExit, response_message, resp);
                        break;
                    }
                    _ => {
                        let err_str = format!("[{}] Unknown action. Received message: {:?}", sessionid, request_message);
                        tracing::warn!("{}", err_str);
                        let response_message = SessionResponseMessage{status_message: err_str, ..SessionResponseMessage::default()};
                        send_response(&sessionid, SessionResponseStatus::SessionOk, response_message, resp);
                        continue;
                    }
                }
            }
        }
    }   
    tracing::info!("[{}] Stopping session loop", sessionid);
    Ok(())
}
