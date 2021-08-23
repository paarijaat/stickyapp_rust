#!/bin/sh
set -ex
# check if server is alive
curl http://localhost:8080/

# list sessions
curl -s http://localhost:8080/sessions | jq

# create normal session
curl -v -s -H 'Content-Type: application/json' http://localhost:8080/sessions -d '{"message": "{}"}' | jq

# create another normal session and store session id
SID=$(curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions -d '{"message": "{}"}' | jq -r '.sessionid')

# list sessions
curl -s http://localhost:8080/sessions | jq

# actions on an existing normal session
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 1}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 2}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 3}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"mean\", \"value\": 0}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"shutdown\", \"value\": 0}"}' | jq
curl -s http://localhost:8080/sessions | jq
sleep 2

# create an encrypted session with default parameters
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions?encrypted=true -d '{"message": "{}"}'

# create an encrypted session with encryption parameters specified
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions?encrypted=true \
-d '{"message": "{\"encoder_min\": 1.0, \"encoder_max\": 99, \"encoder_precision_bits\": 10, \"encoder_padding_bits\": 4, \"secret_key_dimensions\": 1024, \"secret_key_log2_std_dev\": -40}"}'

# create encrypted session and store session id
SID=$(curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions?encrypted=true \
-d '{"message": "{\"encoder_min\": 1.0, \"encoder_max\": 64, \"encoder_precision_bits\": 10, \"encoder_padding_bits\": 4, \"secret_key_dimensions\": 1024, \"secret_key_log2_std_dev\": -40}"}' | jq -r '.sessionid')

# list sessions
curl -s http://localhost:8080/sessions | jq

# action on an existing encrypted session
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 1}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 2}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 3}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 4}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 5}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 6}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 7}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 8}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 9}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"encrypt\", \"value\": 10}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"mean\", \"value\": 0}"}' | jq
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/$SID -d '{"message": "{\"action\": \"shutdown\", \"value\": 0}"}' | jq
curl -s http://localhost:8080/sessions | jq
sleep 2


# action on a non existent session
curl -s -H 'Content-Type: application/json' http://localhost:8080/sessions/acbdefg -d '{"message": "{\"action\": \"encrypt\", \"value\": 1}"}' | jq
# shutdown the server
curl http://localhost:8080/shutdown
