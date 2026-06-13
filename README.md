# OpenMind ESP32 Bridge

**OpenMind ESP32 Bridge** is a Rust motor-neuron bridge connecting the OpenMind muscle memory system to ESP32 hardware. It provides a typed RPC interface over serial (UART/USB) and WebSocket transports with CRC-8 framed packets and ternary GPIO semantics {-1: low, 0: floating, +1: high}.

## Why It Matters

The ESP32 is the world's most popular IoT microcontroller — $4, WiFi+BLE, dual-core. Robotics, agriculture, and industrial sensing all run on it. But high-level AI agent frameworks have no way to directly actuate physical hardware. This bridge closes that loop: an OpenMind conductor can `flex("gpio_write", pin, Trit::PlusOne)` to set a pin high, or `flex("adc_read", pin)` to read analog sensors. The CRC-8 framing ensures data integrity over noisy serial lines, and the ternary GPIO model maps directly to the SuperInstance ternary ecosystem.

## How It Works

### Frame Protocol

All communication uses framed packets with CRC-8-SMBUS integrity checking:

**Command frame (host → ESP32):**
```
[0xAA] [len:u16 LE] [cmd:u8] [payload...] [crc:u8] [0x55]
```

**Response frame (ESP32 → host):**
```
[0xBB] [len:u16 LE] [status:u8] [payload...] [crc:u8] [0x55]
```

CRC computation: **O(N)** per frame (N = payload bytes). Maximum payload: 1024 bytes.

### Transport Abstraction

The `Transport` trait abstracts the physical connection:

```rust
#[async_trait]
trait Transport {
    async fn send_and_recv(&cmd: &Command) -> Result<Response>;
}
```

Two implementations:
- **SerialTransport**: tokio-serial with configurable baud, timeout (default 5s), retries (default 2)
- **WsTransport**: tokio-tungstenite with auto-reconnect

Serial transport includes exponential backoff retry logic. On failure after max retries, returns `BridgeError::Serial`.

### Chord Registry

The bridge uses a static `LazyLock<Vec<Chord>>` registry mapping named operations to command IDs:

| Chord | cmd_id | Params | Return |
|-------|--------|--------|--------|
| `gpio_read` | 0x01 | u8 (pin) | Trit |
| `gpio_write` | 0x02 | u8, Trit | — |
| `spi_transfer` | 0x03 | bytes | bytes |
| `i2c_read` | 0x04 | u8, u8, usize | bytes |
| `i2c_write` | 0x05 | u8, u8, bytes | — |
| `pwm_set` | 0x06 | u8, u16 | — |
| `adc_read` | 0x07 | u8 | u16 |
| `uart_write` | 0x08 | bytes | — |

Argument validation: **O(N)** where N = parameter count. Serialization: little-endian byte packing.

### Ternary GPIO

GPIO pins use ternary logic instead of binary:
- `Trit::MinusOne` (byte 0x00): Pin LOW
- `Trit::Zero` (byte 0x01): Pin FLOATING (high-impedance)
- `Trit::PlusOne` (byte 0x02): Pin HIGH

This maps to the three-state logic used in digital electronics and matches the SuperInstance ternary digit set.

## Quick Start

```rust
use openmind_esp32_bridge::{Esp32Bridge, FlexArg, Trit};

#[tokio::main]
async fn main() {
    let bridge = Esp32Bridge::serial("/dev/ttyUSB0", 115200).await.unwrap();

    // Read GPIO pin 2
    let result = bridge.flex("gpio_read", &[FlexArg::U8(2)]).await.unwrap();

    // Write GPIO pin 4 HIGH
    bridge.flex("gpio_write", &[FlexArg::U8(4), FlexArg::Trit(Trit::PlusOne)]).await.unwrap();

    // Read analog ADC pin 34
    let adc = bridge.flex("adc_read", &[FlexArg::U8(34)]).await.unwrap();
}
```

## API

| Type | Description |
|------|-------------|
| `Esp32Bridge` | High-level bridge with `flex()` method |
| `Transport` | Async trait for serial/WebSocket/mock transports |
| `Command` | cmd_id + payload bytes |
| `Response` | status + payload bytes |
| `FlexArg` | Enum: `U8`, `U16`, `Usize`, `Bytes`, `String`, `Trit` |
| `Trit` | MinusOne, Zero, PlusOne |
| `framing::encode_command()` | Frame a command with CRC |
| `framing::decode_response()` | Parse and verify a response frame |
| `BridgeError` | Serial, WebSocket, Frame, CrcMismatch, DeviceError, etc. |

## Architecture Notes

The ESP32 Bridge is the motor cortex of the OpenMind system in SuperInstance. In γ + η = C, it executes γ (growth — actuating motors and sensors to interact with the physical world) and η (avoidance — the ternary GPIO -1 state actively drives pins low for safety). The bridge connects to `openmind-conductor` via the baton protocol and uses muscle memory patterns for learned hardware sequences.

See [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md) for the OpenMind hardware architecture.

## References

1. Kolban, N. (2018). *Kolban's Book on ESP32*. Leanpub.
2. ANSI X3.4-1967. "Cyclic Redundancy Check (CRC-8-SMBUS) Specification."
3. Tokio Project (2024). "Asynchronous Serial Ports in Rust with tokio-serial."

## License

MIT
