
docker run -d --name rabbit -p 5672:5672 -p 15672:15672 rabbitmq:3-management
# UI de management: http://localhost:15672 (guest/guest)


cargo run

run the sse curl -N http://127.0.0.1:8080/stream


curl -X POST -H "Content-Type: application/json" \
  -d '{"message":"hello via REST -> RMQ"}' \
  http://127.0.0.1:8080/events



npm i -g wscat
wscat -c ws://127.0.0.1:8080/ws
# pour publier côté client WS :
{"message":"hello via WS -> DB -> RMQ"}


use sse from actix:  curl -N http://127.0.0.1:8080/stream-lab


See events: http://127.0.0.1:8080/events


Flow from ws:

WebSocket Input → JSON Parse → Async Task → SQLite Save → RabbitMQ Publish → 
RabbitMQ Consumer → Local Broadcast → WebSocket Output (+ SSE Streams)

Flow from create endpoint:
HTTP POST → JSON Parse → Event Creation → SQLite Save → RabbitMQ Publish → 
HTTP Response → RabbitMQ Consumer → Local Broadcast → All Connected Clients


