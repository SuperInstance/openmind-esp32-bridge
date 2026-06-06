# openmind-esp32-bridge

*Bridge between openmind and ESP32 microcontrollers. Run ternary agents on bare metal — 279 bytes of ternary lookup, 8ns per operation, 520KB of RAM.*

## Why This Exists

The ultimate test of a portable agent framework: can it run on a microcontroller? The ESP32 has 520KB of RAM, a 240MHz dual-core processor, and no operating system. If openmind can bridge to this, it can bridge to anything.

This crate provides the serial and WebSocket transport layers, command framing, type marshaling, and the conductor that orchestrates openmind sessions over the wire. The ESP32 runs the ternary firmware; this crate runs on the host side, translating between openmind's Python API and the ESP32's binary protocol.

## Architecture

```
openmind (Python) ──→ openmind-esp32-bridge (Rust) ──→ Serial/WebSocket ──→ ESP32
                            ↓                              ↓
                      Command framing              Binary protocol
                      Type marshaling               Trit packing
                      Conductor orchestration       Response parsing
```

### Modules

- **`types`** — Shared types: Trit, Command, Response, BridgeError
- **`framing`** — Binary frame encoding/decoding for the serial protocol
- **`transport`** — Abstract transport trait (serial + WebSocket implementations)
- **`serial`** — Serial port transport (UART over USB)
- **`websocket`** — WebSocket transport (for remote/headless ESP32s)
- **`registry`** — Command registry (maps command names to handlers)
- **`conductor`** — Top-level orchestration: manage sessions, route commands, handle errors

### Key Types

- **`Esp32Bridge`** — The conductor. Connect, send commands, receive responses.
- **`FlexArg`** / **`FlexResult`** — Dynamic argument/result types for arbitrary commands
- **`Command`** / **`Response`** — Framed message types

## Usage

```rust
use openmind_esp32_bridge::*;

// Connect via serial
let mut bridge = Esp32Bridge::connect_serial("/dev/ttyUSB0", 115200)?;

// Send a ternary operation
let result = bridge.execute(Command::TernaryOp {
    op: "eval",
    args: vec![FlexArg::Trits(vec![1, -1, 0, 1])],
})?;

match result {
    Response::TritResult(trits) => println!("Result: {:?}", trits),
    Response::Error(msg) => eprintln!("ESP32 error: {}", msg),
    _ => {}
}
```

## The Deeper Idea

The ESP32 bridge proves that the ternary agent architecture is hardware-agnostic. The same ternary logic that runs on an RTX 4050 (6GB VRAM, 20 SMs) also runs on an ESP32 (520KB RAM, 2 cores). The math doesn't change. The representation doesn't change. Only the transport does.

This connects to `ternary-esp32-firmware` (the ESP32-side firmware) and `open-mind-standalone` (the Python agent framework). Together they form the full loop: Python agent → Rust bridge → ESP32 firmware → bare metal ternary computation.

## Related Crates

- `ternary-esp32-firmware` — The ESP32 firmware (the other end of this bridge)
- `open-mind-standalone` — Python agent framework that uses this bridge
- `flux-core` — FLUX bytecode (alternative execution path for agent logic)
- `pincher` — Agent reflexes that can be compiled for ESP32
