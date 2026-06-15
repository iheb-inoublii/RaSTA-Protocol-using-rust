# RaSTA Protocol Stack (Reference/Educational Implementation)

This project is a safety-oriented, `no_std`-capable implementation of the RaSTA
(Rail Safe Transport Application) protocol structure. It separates the
platform-independent protocol layers from platform-dependent transport, clock,
timer, logging, synchronization, and system adapters.

This code is suitable for learning, prototyping, and architecture work. It is
not a certified RaSTA product and still requires conformance vectors, safety
analysis, target-specific validation, and an approved safety process before any
safety-critical use.

## Key Requirements and Implementation

| Requirement | Status | Implementation Detail |
|-------------|--------|-----------------------|
| Safe Rust | Done | No `unsafe` blocks, raw pointers, or manual memory management. |
| No Dynamic Allocation | Done | The core and platform traits are `#![no_std]` capable. Buffers are fixed-size arrays. |
| Portable Architecture | Done | Core logic depends on `Clock`, `Timer`, and `Transport` traits only. |
| Safety and Retransmission Layer | Partial | PDU parsing, little-endian fields, sequence checks, heartbeat, retransmission request/response, timestamp supervision, and configurable safety code. |
| Redundancy Layer | Partial | Redundancy frame, duplicate send, duplicate discard, optional CRC check code. |
| Application API | Partial | `open_connection`, `send_data`, `receive_data`, `close_connection`, `status`, and `poll`. |
| Unit Testing | Partial | Tests cover packets, safety-code MD4 vectors, timestamp supervision, sequence checks, state transitions, retransmission buffer, application receive queue, and redundant duplicate discard. |

## Layer Split

```text
Railway Signalling Application / SCI
    |
Application API / RaSTA Service    src/application/service_interface.rs
    |
Platform-Independent RaSTA Core    src/core/*
    |                               SRL + RL protocol logic
    |
Abstract Platform Interfaces       src/platform/*
    |
Platform-Specific Adapters         src/adapters/*
    |
Target Platform / System Services  OS, network, timers, clocks, hardware
```

The protocol core does not import UDP, TCP, sockets, OS time, threads, or heap
allocation. Those details are supplied through traits.

## Project Structure

- `src/application/service_interface.rs`: RaSTA service API used by the railway signalling application.
- `src/core/`: platform-independent SRL/RL protocol logic.
  - `connection_state_machine.rs`
  - `pdu.rs`
  - `sequencing.rs`
  - `retransmission.rs`
  - `safety_code.rs`
  - `heartbeat.rs`
  - `redundancy_management.rs`
  - `time_supervision.rs`
- `src/platform/`: abstract platform interfaces only.
  - `transport.rs`
  - `timer.rs`
  - `clock.rs`
  - `logger.rs`
  - `synchronization.rs`
- `src/adapters/`: platform-dependent implementations.
  - `socket_transport.rs`
  - `standard_timer.rs`
  - `standard_clock.rs`
  - `embedded_ethernet.rs`
  - `linux.rs`
  - `windows.rs`
  - `test.rs`
- `src/tests.rs`: automated unit tests.

## Build and Test

```bash
cargo check
cargo check --features std
cargo test
cargo run --features std --bin rasta_node -- A 127.0.0.1
```

## Safety-Code Note

Safety and Retransmission Layer packets support the RaSTA safety-code modes:

- no safety code,
- lower 8 bytes of MD4,
- full 16 bytes of MD4.

The MD4 initial value is configurable and is used to separate RaSTA networks.
The default is the standard MD4 initial value with the lower 8-byte safety code.
