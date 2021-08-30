use crate::session_common::*;
use concrete::*;
use serde::{Deserialize};

#[derive(Deserialize,Debug)]
#[serde(default)]
struct EncryptionParameters { 
    encoder_min: f64,
    encoder_max: f64,
    encoder_precision_bits: usize,
    encoder_padding_bits: usize,
    secret_key_dimensions: usize,
    secret_key_log2_std_dev: i32
}
impl Default for EncryptionParameters {
    fn default() -> Self { 
        Self {
            encoder_min: 0.,
            encoder_max: 100.,
            encoder_precision_bits: 16,
            encoder_padding_bits: 4,
            secret_key_dimensions: 1024,
            secret_key_log2_std_dev: -40,
        }
    }
}

pub async fn spawn(
    sessionid: String, init_message: String,
    request_channel_rx: ReceiverSessionRequestChannel,
    init_success_tx: SenderSessionResponseChannel,
) -> () {
    tracing::info!("[{}] Spawning encryted session", sessionid);
    // launch the session loop as a tokio task
    tokio::spawn(session_loop(sessionid.clone(), request_channel_rx, init_message, init_success_tx));
}

async fn session_loop(
    sessionid: String, 
    mut request_channel_rx: ReceiverSessionRequestChannel,
    init_message: String,
    init_success_tx: SenderSessionResponseChannel,
) -> Result<(), ()> {
    tracing::debug!("[{}] Starting session loop", sessionid);

    let encryption_parameters: EncryptionParameters = match serde_json::from_str(init_message.as_str()) {
        Ok(m) => m,
        Err(e) => {
            let status_message = format!("[{}] Session initialized failed. Failed to json decode encryption parameters. {}", sessionid, e.to_string());
            tracing::warn!("{}", status_message);
            send_response(
                &sessionid, 
                SessionResponseStatus::SessionExit, 
                SessionResponseMessage{status_message, ..SessionResponseMessage::default()}, 
                init_success_tx
            );
            return Err(());
        }
    };
    tracing::debug!("[{}] Encryption parameters: {:?}", sessionid, encryption_parameters);
    // https://github.com/zama-ai/concrete/blob/831095337c7003cc9ddb832c1c85a1455993cd49/concrete/src/lwe_secret_key.rs#L29
    // https://github.com/zama-ai/concrete/blob/831095337c7003cc9ddb832c1c85a1455993cd49/concrete/src/lwe_params.rs
    let secret_key_params = LWEParams::new (
        encryption_parameters.secret_key_dimensions, encryption_parameters.secret_key_log2_std_dev
    );
    let secret_key = LWESecretKey::new(&secret_key_params);

    // https://github.com/zama-ai/concrete/blob/831095337c7003cc9ddb832c1c85a1455993cd49/concrete/src/encoder/mod.rs#L59
    // fn Encoder::new(..) -> Result<Encoder, CryptoAPIError>
    let encoder = match Encoder::new(
        encryption_parameters.encoder_min, 
        encryption_parameters.encoder_max, 
        encryption_parameters.encoder_precision_bits, 
        encryption_parameters.encoder_padding_bits
    ) {
        Ok(enc) => {
            let status_message = format!("[{}] Session initialized, with parameters: {:?}", sessionid, encryption_parameters);
            tracing::info!("{}", status_message);
            if !send_response(
                &sessionid, 
                SessionResponseStatus::SessionOk, 
                SessionResponseMessage{status_message, status: true, ..SessionResponseMessage::default()}, 
                init_success_tx
            ) {
                return Err(())
            }
            enc
        }
        Err(err) => {
            let status_message = format!("[{}] Session initialized failed. Unable to instantiate encoder. {}", sessionid, err);
            tracing::warn!("{}", status_message);
            send_response(
                &sessionid,
                SessionResponseStatus::SessionExit,
                SessionResponseMessage{status_message, ..SessionResponseMessage::default()},
                init_success_tx
            );
            return Err(());
        }
    }; 

    // Init has succeeded. Start main message loop
    let mut values: Vec<f64> = Vec::new();
    let mut values_encrypted: Vec<LWE> = Vec::new();

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
                            SessionResponseMessage{status_message, ..SessionResponseMessage::default()},
                            resp
                        );
                        continue;
                    }
                };
                
                tracing::debug!("[{}] Received message: {:?}", sessionid, request_message);

                match request_message.action.as_str() {
                    "encrypt" => {
                        tracing::debug!("[{}] Encrypt action received. Value {}", sessionid, request_message.value);
                        let mut response_message = SessionResponseMessage::default();

                        // https://github.com/zama-ai/concrete/blob/831095337c7003cc9ddb832c1c85a1455993cd49/concrete/src/lwe/mod.rs#L113
                        // fn LWE::encode_encrypt(.., f64, ..) -> Result<LWE, CryptoAPIError>
                        let encrypted_val = LWE::encode_encrypt(
                            &secret_key, 
                            request_message.value, 
                            &encoder);
                        
                        match encrypted_val {
                            Ok(eval) => {
                                let msg_str = format!("[{}] Encrypt action, Value {} encrypted successfully", sessionid, request_message.value);
                                tracing::debug!("{}", msg_str);
                                response_message.status = true;
                                response_message.status_message = msg_str;
                                response_message.value = request_message.value;
                                values_encrypted.push(eval);
                                values.push(request_message.value); 
                            }
                            Err(e) => {
                                let err_str = format!("[{}] Failed to encrypt value. {}", sessionid, e);
                                tracing::warn!("{}", err_str);
                                response_message.status = false;
                                response_message.status_message = err_str;    
                            }
                        }
                        send_response(&sessionid, SessionResponseStatus::SessionOk, response_message, resp);
                        continue;
                    }
                    "mean" => {
                        let msg_str = format!("[{}] Mean action received", sessionid);
                        tracing::debug!("{}", msg_str);
                        let mut response_message = SessionResponseMessage{status: true, ..SessionResponseMessage::default()};

                        // compute the orignal sum and mean for debugging
                        if values.len() > 0 {
                            let mut original_sum: f64 = 0.;
                            for value in &values {
                                original_sum += value;
                            };
                            let length_multiplier =  1. / (values.len() as f64);
                            let original_mean = original_sum * length_multiplier;

                            // calculate the mean over encrypted values
                            let mut encrypted_sum: LWE = values_encrypted.pop().unwrap();
                            for encrypted_value in &values_encrypted {
                                // https://github.com/zama-ai/concrete/blob/831095337c7003cc9ddb832c1c85a1455993cd49/concrete/src/lwe/mod.rs#L770
                                //if let Err(e) = encrypted_sum.add_with_padding_inplace(encrypted_value) {
                                if let Err(e) = encrypted_sum.add_with_new_min_inplace(encrypted_value, 0.0) {
                                    let err_str = format!("[{}] Mean action, Failed to add two encrypted values. {}", sessionid, e);
                                    tracing::warn!("{}", err_str);
                                    response_message.status_message = err_str;
                                    response_message.status = false;
                                    break;
                                }
                            }
                            
                            // try to decrypt the sum, for debugging
                            let decrypted_sum: f64 = 0.;
                            /*
                            if response_message.status {
                                match encrypted_sum.decrypt_decode(&secret_key) {
                                    Ok(dsum) => decrypted_sum = dsum,
                                    Err(e) => {
                                        let err_str = format!("[{}] Mean action, Failed to decrypt sum. {}", sessionid, e);
                                        tracing::warn!("{}", err_str);
                                        response_message.status_message = err_str;
                                        response_message.status = false;
                                    }
                                }
                            }
                            */

                            // Calculate the encrypted mean
                            if response_message.status {
                                let max_constant: f64 = 1.;
                                let nb_bit_padding = 4;
                                if let Err(e) = encrypted_sum.mul_constant_with_padding_inplace(length_multiplier, max_constant, nb_bit_padding) {
                                    let err_str = format!("[{}] Mean action, Failed to multiply encrypted sum with a float value. {}", sessionid, e);
                                    tracing::warn!("{}", err_str);
                                    response_message.status_message = err_str;
                                    response_message.status = false;
                                }
                            }

                            let mut decrypted_mean: f64 = 0.;
                            if response_message.status {
                                match encrypted_sum.decrypt_decode(&secret_key) {
                                    Ok(dmean) => decrypted_mean = dmean,
                                    Err(e) => {
                                        let err_str = format!("[{}] Mean action, Failed to decrypt mean. {}", sessionid, e);
                                        tracing::warn!("{}", err_str);
                                        response_message.status_message = err_str;
                                        response_message.status = false;
                                    }
                                }
                            }

                            if response_message.status {
                                let msg_str = format!(
                                    "[{}] Mean action, Mean calculated successfully. Orignal sum: {}, Original mean: {}. Decrypted sum: {}, Decrypted mean: {}", 
                                    sessionid,
                                    original_sum,
                                    original_mean,
                                    decrypted_sum,
                                    decrypted_mean,
                                );
                                tracing::debug!("{}", msg_str);
                                response_message.value = decrypted_mean;
                                response_message.status_message = msg_str
                            }
                            values.clear();
                            values_encrypted.clear();
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
