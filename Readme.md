
# Event Bus with Actix, RabbitMQ, and SQLx

A high-performance Rust event bus system built with Actix Web, RabbitMQ, and SQLx that demonstrates real-time communication patterns through WebSockets, Server-Sent Events (SSE), and REST APIs.

## Architecture

This application implements an event-driven architecture with the following flow:

### WebSocket Flow
```
WebSocket Input → JSON Parse → Async Task → SQLite Save → RabbitMQ Publish → 
RabbitMQ Consumer → Local Broadcast → WebSocket Output (+ SSE Streams)
```

### REST API Flow
```
HTTP POST → JSON Parse → Event Creation → SQLite Save → RabbitMQ Publish → 
HTTP Response → RabbitMQ Consumer → Local Broadcast → All Connected Clients
```

## Quick Start

### 1. Start RabbitMQ
```bash
docker run -d --name rabbit -p 5672:5672 -p 15672:15672 rabbitmq:3-management
```
- Management UI: http://localhost:15672 (guest/guest)

### 2. Run the Application
```bash
cargo run
```

## API Endpoints

### Events

#### Create Event
```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"message":"hello via REST -> RMQ"}' \
  http://127.0.0.1:8080/events
```

#### Get All Events
```bash
curl -X GET http://127.0.0.1:8080/events
```

#### Server-Sent Events Stream
```bash
# Standard SSE stream
curl -N http://127.0.0.1:8080/stream

# Alternative SSE stream
curl -N http://127.0.0.1:8080/stream-lab
```

### CAN Bus Data

#### Create CAN Data
```bash
curl -X POST http://localhost:8080/can \
  -H "Content-Type: application/json" \
  -d '{
    "id": 1234,
    "speed": 80,
    "temperature": 65,
    "pressure": 850
  }'
```

#### Get CAN Data
```bash
curl -X GET http://localhost:8080/can \
  -H "Content-Type: application/json"
```

## WebSocket Usage

### Setup wscat (if not installed)
```bash
npm i -g wscat
```

### Connect to WebSocket
```bash
wscat -c ws://127.0.0.1:8080/ws
```

### Send Message via WebSocket
```json
{"message":"hello via WS -> DB -> RMQ"}
```

## Features

- **Real-time Communication**: WebSocket and SSE support for live updates
- **Event Bus Architecture**: Decoupled message handling with RabbitMQ
- **Data Persistence**: SQLite integration with SQLx
- **CAN Bus Simulation**: Automotive data handling endpoints
- **High Performance**: Built with Actix Web for concurrent request handling

## Technology Stack

- **Actix Web**: High-performance web framework
- **RabbitMQ**: Message broker for event distribution
- **SQLx**: Async SQL toolkit with compile-time verification
- **SQLite**: Lightweight database for data persistence
- **Tokio**: Async runtime for Rust