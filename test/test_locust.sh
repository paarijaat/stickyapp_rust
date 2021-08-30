#!/bin/sh
locust -f profile-stickyapp_rust.py --host http://stickyapp-rust.10.0.2.15.sslip.io --headless --print-stats --only-summary --csv stickyapp_rust --loglevel DEBUG --u 1 -r 1 -t 10