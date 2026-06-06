use std::sync::LazyLock;
use crate::types::{Chord, Decision, ParamType, Signature};

fn chords() -> &'static Vec<Chord> {
    static CHORDS: LazyLock<Vec<Chord>> = LazyLock::new(|| vec![
        Chord {
            name: "gpio_read",
            cmd_id: 0x01,
            signature: Signature {
                params: vec![ParamType::U8],
                ret: Some(ParamType::Trit),
            },
            decision: Decision::Hardcode,
            description: "Read a GPIO pin as {-1: low, 0: floating, +1: high}",
        },
        Chord {
            name: "gpio_write",
            cmd_id: 0x02,
            signature: Signature {
                params: vec![ParamType::U8, ParamType::Trit],
                ret: None,
            },
            decision: Decision::Hardcode,
            description: "Write a GPIO pin",
        },
        Chord {
            name: "spi_transfer",
            cmd_id: 0x03,
            signature: Signature {
                params: vec![ParamType::Bytes],
                ret: Some(ParamType::Bytes),
            },
            decision: Decision::Hardcode,
            description: "SPI transaction",
        },
        Chord {
            name: "i2c_read",
            cmd_id: 0x04,
            signature: Signature {
                params: vec![ParamType::U8, ParamType::U8, ParamType::Usize],
                ret: Some(ParamType::Bytes),
            },
            decision: Decision::Hardcode,
            description: "I2C read from address and register",
        },
        Chord {
            name: "i2c_write",
            cmd_id: 0x05,
            signature: Signature {
                params: vec![ParamType::U8, ParamType::U8, ParamType::Bytes],
                ret: None,
            },
            decision: Decision::Hardcode,
            description: "I2C write to address and register",
        },
        Chord {
            name: "pwm_set",
            cmd_id: 0x06,
            signature: Signature {
                params: vec![ParamType::U8, ParamType::U16],
                ret: None,
            },
            decision: Decision::Hardcode,
            description: "Set PWM duty cycle on a pin",
        },
        Chord {
            name: "adc_read",
            cmd_id: 0x07,
            signature: Signature {
                params: vec![ParamType::U8],
                ret: Some(ParamType::U16),
            },
            decision: Decision::Hardcode,
            description: "Read analog value from ADC pin",
        },
        Chord {
            name: "uart_write",
            cmd_id: 0x08,
            signature: Signature {
                params: vec![ParamType::Bytes],
                ret: None,
            },
            decision: Decision::Hardcode,
            description: "Transmit data over UART",
        },
        Chord {
            name: "mqtt_publish",
            cmd_id: 0x09,
            signature: Signature {
                params: vec![ParamType::String, ParamType::Bytes],
                ret: None,
            },
            decision: Decision::Hardcode,
            description: "Publish to MQTT topic",
        },
        Chord {
            name: "mqtt_subscribe",
            cmd_id: 0x0A,
            signature: Signature {
                params: vec![ParamType::String],
                ret: Some(ParamType::Bytes),
            },
            decision: Decision::Hardcode,
            description: "Subscribe to MQTT topic, returns first message",
        },
    ]);
    &CHORDS
}

/// Look up a chord by name.
pub fn lookup(name: &str) -> Option<&'static Chord> {
    chords().iter().find(|c| c.name == name)
}

/// Get all chord names.
pub fn names() -> Vec<&'static str> {
    chords().iter().map(|c| c.name).collect()
}

/// Get all chords.
pub fn all() -> &'static Vec<Chord> {
    chords()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_chords_present() {
        assert_eq!(all().len(), 10);
    }

    #[test]
    fn lookup_by_name() {
        let ch = lookup("gpio_read").unwrap();
        assert_eq!(ch.cmd_id, 0x01);
        assert_eq!(ch.signature.params.len(), 1);
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(lookup("nonexistent").is_none());
    }

    #[test]
    fn all_chords_are_hardcode() {
        for ch in all().iter() {
            assert_eq!(ch.decision, Decision::Hardcode, "{} not Hardcode", ch.name);
        }
    }

    #[test]
    fn signature_parsing() {
        let gpio_write = lookup("gpio_write").unwrap();
        assert_eq!(gpio_write.signature.params[0], ParamType::U8);
        assert_eq!(gpio_write.signature.params[1], ParamType::Trit);
        assert!(gpio_write.signature.ret.is_none());

        let spi = lookup("spi_transfer").unwrap();
        assert_eq!(spi.signature.params[0], ParamType::Bytes);
        assert_eq!(spi.signature.ret, Some(ParamType::Bytes));
    }

    #[test]
    fn all_names_unique() {
        let mut seen = std::collections::HashSet::new();
        for name in names() {
            assert!(seen.insert(name), "duplicate chord name: {name}");
        }
    }

    #[test]
    fn all_cmd_ids_unique() {
        let mut seen = std::collections::HashSet::new();
        for ch in all().iter() {
            assert!(seen.insert(ch.cmd_id), "duplicate cmd_id: {:02X}", ch.cmd_id);
        }
    }
}
