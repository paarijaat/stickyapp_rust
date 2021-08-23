# Sticky app (rust)

An implementation of sticky sessions in Rust + Sticky routing using Istio and Envoy. Allows for creating persistent sessions and send messages to to them.

It uses Istio + Envoy to implement sticky routing, where session creation requests are load balanced as usual, but sunsequent requests can add routed to a specific pod by specifying an http header.