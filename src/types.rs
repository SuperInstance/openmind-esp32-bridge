use std::fmt;

/// Ternary digit: -1 (low), 0 (floating), +1 (high)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Trit {
    MinusOne,
    Zero,
    PlusOne,
}

impl Trit {
    pub fn from_int(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Trit::MinusOne),
            0 => Some(Trit::Zero),
            1 => Some(Trit::PlusOne),
            _ => None,
        }
    }

    pub fn to_int(self) -> i8 {
        match self {
            Trit::MinusOne => -1,
            Trit::Zero => 0,
            Trit::PlusOne => 1,
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Trit::MinusOne => 0,
            Trit::Zero => 1,
            Trit::PlusOne => 2,
        }
    }

    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Trit::MinusOne),
            1 => Some(Trit::Zero),
            2 => Some(Trit::PlusOne),
            _ => None,
        }
    }
}

impl From<i8> for Trit {
    fn from(v: i8) -> Self {
        Trit::from_int(v).expect("Trit value must be -1, 0, or 1")
    }
}

impl fmt::Display for Trit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_int())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ParamType {
    U8,
    U16,
    Usize,
    Bytes,
    String,
    Trit,
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamType::U8 => write!(f, "u8"),
            ParamType::U16 => write!(f, "u16"),
            ParamType::Usize => write!(f, "usize"),
            ParamType::Bytes => write!(f, "&[u8]"),
            ParamType::String => write!(f, "&str"),
            ParamType::Trit => write!(f, "Trit"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Signature {
    pub params: Vec<ParamType>,
    pub ret: Option<ParamType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Decision {
    Hardcode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Chord {
    pub name: &'static str,
    pub cmd_id: u8,
    pub signature: Signature,
    pub decision: Decision,
    pub description: &'static str,
}

/// Command sent to ESP32
#[derive(Debug, Clone)]
pub struct Command {
    pub cmd_id: u8,
    pub payload: Vec<u8>,
}

/// Response from ESP32
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u8,
    pub payload: Vec<u8>,
}

impl Response {
    pub const OK: u8 = 0x00;
    pub const ERROR: u8 = 0x01;
    pub const UNKNOWN_CMD: u8 = 0x02;

    pub fn is_ok(&self) -> bool {
        self.status == Self::OK
    }
}

/// Protocol framing constants
pub mod frame {
    pub const CMD_HEADER: u8 = 0xAA;
    pub const RSP_HEADER: u8 = 0xBB;
    pub const FOOTER: u8 = 0x55;
    pub const MAX_PAYLOAD: usize = 1024;
}

#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Serial error: {0}")]
    Serial(String),
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    #[error("Transport disconnected")]
    Disconnected,
    #[error("Timeout after {0}ms")]
    Timeout(u64),
    #[error("Frame error: {0}")]
    Frame(String),
    #[error("Unknown chord: {0}")]
    UnknownChord(String),
    #[error("Wrong arguments for chord '{name}': {detail}")]
    WrongArgs { name: String, detail: String },
    #[error("ESP32 error status {status}: {detail:?}")]
    DeviceError { status: u8, detail: String },
    #[error("CRC mismatch: expected {expected:02X}, got {actual:02X}")]
    CrcMismatch { expected: u8, actual: u8 },
}

pub type Result<T> = std::result::Result<T, BridgeError>;
