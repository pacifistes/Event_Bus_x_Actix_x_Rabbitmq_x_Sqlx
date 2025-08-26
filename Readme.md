
# Event Bus with Actix, RabbitMQ, and SQLx

A high-performance Rust event bus system built with Actix Web, RabbitMQ, and SQLx that simulates automotive CAN bus data processing with real-time communication patterns through WebSockets, Server-Sent Events (SSE), and REST APIs.

## Architecture

This application implements an event-driven architecture simulating automotive data processing:

### CAN Bus Simulation Flow
```
DrivingStep → 7 CAN Messages → SQLite Storage → RabbitMQ → 
RabbitMQ Consumer → CAN Reconstruction → Broadcast → WebSocket/SSE Clients
```

### WebSocket Flow
```
WebSocket Input (DrivingStep JSON) → CAN Conversion → SQLite Save → RabbitMQ Publish → 
RabbitMQ Consumer → CAN Reconstruction → Local Broadcast → WebSocket Output (+ SSE Streams)
```

### REST API Flow
```
HTTP GET → CAN Messages Fetch → DrivingStep Reconstruction → JSON Response
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

### Driving Steps (Reconstructed from CAN Messages)

#### Get All Driving Steps
```bash
curl -X GET http://127.0.0.1:8080/driving-steps
```
Returns all driving steps reconstructed from CAN messages stored in the database.

#### Get Latest Driving Step
```bash
curl -X GET http://127.0.0.1:8080/driving-steps/last
```
Returns the most recent driving step reconstructed from the latest 7 CAN messages.

#### Server-Sent Events Stream
```bash
# Standard SSE stream
curl -N http://127.0.0.1:8080/stream

# Enhanced SSE stream with actix-web-lab
curl -N http://127.0.0.1:8080/stream-lab
```
Real-time stream of driving steps as they are processed through the RabbitMQ pipeline.

## WebSocket Usage

### Setup wscat (if not installed)
```bash
npm i -g wscat
```

### Connect to WebSocket
```bash
wscat -c ws://127.0.0.1:8080/ws
```

### Send Driving Step via WebSocket
```bash
wscat -c ws://127.0.0.1:8080/ws -x '{"step_name":"OneCommand_Test","engine":{"rpm":1500,"coolant_temp":75,"throttle_pos":30,"engine_load":25,"intake_temp":28,"fuel_pressure":320,"engine_running":true},"speed":{"vehicle_speed":60.0,"gear_position":4,"wheel_speeds":[60.1,60.2,60.0,60.3],"abs_active":false,"traction_control":true,"cruise_control":false},"climate":{"cabin_temp":20,"target_temp":21,"outside_temp":16,"fan_speed":40,"ac_compressor":false,"heater":false,"defrost":false,"auto_mode":true,"air_recirculation":false},"duration_ms":1500}'
````

This will:
1. Convert the DrivingStep to 7 CAN messages
2. Store CAN messages in SQLite
3. Publish step_name to RabbitMQ
4. Trigger reconstruction and broadcast to all connected clients

## Features

- **Automotive CAN Bus Simulation**: Convert driving data to/from CAN messages (7 messages per driving step)
- **Real-time Communication**: WebSocket and SSE support for live updates
- **Event Bus Architecture**: Decoupled message handling with RabbitMQ
- **Data Reconstruction**: Rebuild complete driving steps from stored CAN messages
- **Async Processing**: Non-blocking I/O with Tokio spawn for concurrent operations
- **High Performance**: Built with Actix Web for concurrent request handling

## CAN Message Structure

Each `DrivingStep` is converted to exactly 7 CAN messages with specific IDs:
- `0x100` - Engine RPM, fuel pressure, engine running status
- `0x101` - Engine temperatures, throttle position, engine load
- `0x200` - Vehicle speed, gear position, wheel speeds
- `0x201` - ABS, traction control, cruise control flags
- `0x300` - Cabin, target, and outside temperatures
- `0x301` - Fan speed and climate control flags
- `0x400` - Step duration and step name hash

## Example Scenario

Run the complete driving scenario example:
```bash
cargo run --example complete_driving_scenario
```

This will:
1. Set up database and RabbitMQ connections
2. Run through a complete driving scenario (startup → acceleration → cruising → deceleration → parking)
3. Convert each step to CAN messages and store them
4. Demonstrate the full pipeline with real automotive data

## Technology Stack

- **Actix Web**: High-performance web framework
- **RabbitMQ**: Message broker for event distribution
- **SQLx**: Async SQL toolkit with compile-time verification  
- **SQLite**: Lightweight database for CAN message persistence
- **Tokio**: Async runtime for Rust
- **Serde**: JSON serialization/deserialization
- **Chrono**: Date and time handling for timestamps