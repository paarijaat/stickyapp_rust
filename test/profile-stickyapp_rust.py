import random
import json
from locust import task, between, HttpUser
from locust.contrib.fasthttp import FastHttpUser
from profile_utils import ProfileUtils
import logging

"""
Simple benchmarking profile built for stickyapp_rust
"""

class HeStickyAppRust(HttpUser):
    wait_time = between(1, 3)

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.host = "stickyapp-rust.10.0.2.15.sslip.io"
        #self.host = "localhost:8080"
        self.podport = 8080
        self._utils = ProfileUtils(self, self.host)
        self.logger = self._utils.get_logger()
        self.logger.setLevel(logging.DEBUG)
        self._num_messages_sent = 0
        self._num_responses_recv = 0
        self.sessionid = None
        self.sessionlocation = None

    def on_start(self):
        self.logger.info("User is starting")
        self.wf_setup_session()

    def on_stop(self):
        self.logger.info(f"Session [{self.sessionid},{self.sessionlocation}] User is ending")
        self.wf_remove_session()
        self.logger.info(f"Session [{self.sessionid},{self.sessionlocation}] User sent messages: " + str(self._num_messages_sent))
        self.logger.info(f"Session [{self.sessionid},{self.sessionlocation}] User received messages: " + str(self._num_responses_recv))

    def wf_setup_session(self):
        # curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions?encrypted=true \
        # -d '{"message": "{\"encoder_min\": 1.0, \"encoder_max\": 99, \"encoder_precision_bits\": 10, \"encoder_padding_bits\": 4, \"secret_key_dimensions\": 1024, \"secret_key_log2_std_dev\": -40}"}'
        encryption_params = {
            "encoder_min": 0.0,
            "encoder_max": 99.0,
            "encoder_precision_bits": 16,
            "encoder_padding_bits": 6,
            "secret_key_dimensions": 1024,
            "secret_key_log2_std_dev": -40
        }
        path = "/sessions?encrypted=true"

        command_response, raw = self.send_session_command(encryption_params,path)

        if type(command_response) == type({}) and command_response['status'] == True:
            self.sessionid = raw.headers["x-sessionid"]
            self.sessionlocation = raw.headers["x-sessionlocation"]
            command_headers = self._utils.get_headers()
            command_headers["use-direct"] = "true"
            command_headers["x-envoy-original-dst-host"] = self.sessionlocation + ":" + str(self.podport)
            self.logger.debug(f"Session [{self.sessionid},{self.sessionlocation}] created. Message: {command_response['status_message']}, Headers set to: " + str(self._utils.get_headers()))
        else:
            self.logger.warn(f"Session [{self.sessionid},{self.sessionlocation}] Initialization Failed. {command_response}")


    def wf_remove_session(self):
        session_command = {
            "action": "shutdown",
            "value": 0.0
        }

        command_response, _ = self.send_session_command(session_command)

        if type(command_response) == type({}) and command_response['status'] == True:
            self.logger.debug(f"Session [{self.sessionid},{self.sessionlocation}] {session_command['action']}: Success. Message: {command_response['status_message']}")
        else:
            self.logger.warn(f"Session [{self.sessionid},{self.sessionlocation}] {session_command['action']}: Failed. {command_response}")

    @task(10)
    def wf_enc_request(self):
        # curl -H "use-direct: true" -H "x-envoy-original-dst-host: 10.42.0.37:8080" http://stickyapp-rust.10.0.2.15.sslip.io/sessions
        session_command = {
            "action": "encrypt",
            "value": random.randrange(10, 90) * 1.0
        }

        command_response, _ = self.send_session_command(session_command)

        if type(command_response) == type({}) and command_response['status'] == True:
            self.logger.debug(f"Session [{self.sessionid},{self.sessionlocation}] {session_command['action']}: Success. Message: {command_response['status_message']}")
        else:
            self.logger.warn(f"Session [{self.sessionid},{self.sessionlocation}] {session_command['action']}: Failed. {command_response}")

    @task(1)
    def wf_get_mean(self):
        session_command = {
            "action": "mean",
            "value": 0.0
        }
        command_response, _ = self.send_session_command(session_command)
        '''
        {
            "status": <bool>,
            "status_message": <String>,
            "value": <f64>
        }
        '''
        if type(command_response) == type({}) and command_response['status'] == True:
            self.logger.debug(f"Session [{self.sessionid},{self.sessionlocation}] {session_command['action']}: Success. Value: {command_response['value']}, message: {command_response['status_message']}")
        else:
            self.logger.warn(f"Session [{self.sessionid},{self.sessionlocation}] {session_command['action']}: Failed. {command_response}")

    def send_session_command(self, session_command, path=None):
        message = {"message": json.dumps(session_command)}
        if path == None:
            path = "/sessions/" + str(self.sessionid)
        self.logger.debug(f"Session [{self.sessionid},{self.sessionlocation}] Sending to {path}: {str(session_command)}")
        r = self.send_message(message,path)
        command_response = r.text
        try:
            if r.status_code == 200:
                response = r.json()
                command_response = response['message']
                command_response = json.loads(response['message'])
                '''
                {
                    status: bool,
                    message: String, <This message is json encoded if status == True, else it is an error message>
                    sessionid: String,
                }
                '''
        except Exception as e:
            command_response = command_response + ', ' + str(e)

        self.logger.debug(f"Session [{self.sessionid},{self.sessionlocation}] Response: {str(command_response)}")
        return command_response, r

    def send_message(self, payload, path):
        self._num_messages_sent += 1
        response = self._utils.send_message(payload, path)
        self._num_responses_recv += 1
        return response
