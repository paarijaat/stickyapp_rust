#!/bin/sh
locust -f profile-stickyapp_rust.py --host http://localhost:8080 --headless --print-stats --only-summary --csv stickyapp_rust --loglevel INFO --u 1 -r 1 -t 200