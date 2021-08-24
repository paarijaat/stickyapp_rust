import logging
import os
from http.client import HTTPConnection

class ProfileUtils:
    def __init__(self, user, host):
        self._user = user
        self._logger = logging.getLogger(self._user.__class__.__name__)
        self._logger.setLevel(logging.INFO)
        self._host = host
        self._headers = {
            "Content-Type": "application/json",
            "Host" : self._host
        }
        self._request_timeout = 100.0

        self._init_proxies()

        #self._setup_http_debug_log()

        #print(self._proxies)
        #print(self._headers)

    def _init_proxies(self):
        http_proxy = ""
        if os.getenv('http_proxy') is not None:
          http_proxy = os.getenv('http_proxy')

        https_proxy = ""
        if os.getenv('https_proxy') is not None:
          https_proxy = os.getenv('https_proxy')

        if https_proxy == "":
            https_proxy = http_proxy

        self._proxies = {
            "http": http_proxy,
            "https": http_proxy
        }

    def _setup_http_debug_log(self):
        logging.basicConfig()
        logging.getLogger().setLevel(logging.DEBUG)

        HTTPConnection.debuglevel = 0
        requests_log = logging.getLogger("requests.packages.urllib3")
        requests_log.setLevel(logging.DEBUG)
        requests_log.propagate = True

    def get_headers(self):
        return self._headers

    def get_proxies(self):
        return self._proxies

    def get_logger(self):
        return self._logger

    def send_message(self, payload, path="/"):
        self._logger.debug(f"Sending message to {path}: {str(payload)[0:200]}")
        #response = self._user.client.post(path, json=payload, proxies=self._proxies, headers=self._headers, timeout=self._request_timeout)
        response = self._user.client.post(path, json=payload, headers=self._headers)
        return response

# locust -f profile-stickyapp_rust.py --host http://localhost:8080 --headless --only-summary --loglevel DEBUG --u 1 -r 1 -t 5