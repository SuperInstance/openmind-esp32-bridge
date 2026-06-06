use crc::{Crc, CRC_8_SMBUS};
use crate::types::{frame, BridgeError, Command, Response, Result};

const CRC_ALG: Crc<u8> = Crc::<u8>::new(&CRC_8_SMBUS);

fn compute_crc(data: &[u8]) -> u8 {
    let mut digest = CRC_ALG.digest();
    digest.update(data);
    digest.finalize()
}

/// Encode a command into a framed byte buffer.
/// Frame: [0xAA][len:u16 LE][cmd:u8][payload][crc:u8][0x55]
pub fn encode_command(cmd: &Command) -> Result<Vec<u8>> {
    if cmd.payload.len() > frame::MAX_PAYLOAD {
        return Err(BridgeError::Frame(format!(
            "payload too large: {} > {}",
            cmd.payload.len(),
            frame::MAX_PAYLOAD
        )));
    }

    let inner_len = 1 + cmd.payload.len();
    let total_len = 1 + 2 + inner_len + 1 + 1;

    let mut buf = Vec::with_capacity(total_len);
    buf.push(frame::CMD_HEADER);
    buf.extend_from_slice(&(inner_len as u16).to_le_bytes());
    buf.push(cmd.cmd_id);
    buf.extend_from_slice(&cmd.payload);

    let crc_val = compute_crc(&buf[1..]);
    buf.push(crc_val);
    buf.push(frame::FOOTER);

    Ok(buf)
}

/// Decode a response from a framed byte buffer.
/// Frame: [0xBB][len:u16 LE][status:u8][payload][crc:u8][0x55]
pub fn decode_response(data: &[u8]) -> Result<Response> {
    if data.len() < 6 {
        return Err(BridgeError::Frame(format!(
            "response too short: {} bytes",
            data.len()
        )));
    }

    if data[0] != frame::RSP_HEADER {
        return Err(BridgeError::Frame(format!(
            "bad header: {:02X}, expected {:02X}",
            data[0], frame::RSP_HEADER
        )));
    }

    let inner_len = u16::from_le_bytes([data[1], data[2]]) as usize;
    if data.len() < 3 + inner_len + 2 {
        return Err(BridgeError::Frame("response truncated".into()));
    }

    let status = data[3];
    let payload = data[4..4 + inner_len - 1].to_vec();

    let crc_idx = 3 + inner_len;
    let expected_crc = compute_crc(&data[1..crc_idx]);
    let actual_crc = data[crc_idx];

    if expected_crc != actual_crc {
        return Err(BridgeError::CrcMismatch {
            expected: expected_crc,
            actual: actual_crc,
        });
    }

    if data[crc_idx + 1] != frame::FOOTER {
        return Err(BridgeError::Frame(format!(
            "bad footer: {:02X}, expected {:02X}",
            data[crc_idx + 1],
            frame::FOOTER
        )));
    }

    Ok(Response { status, payload })
}

/// Encode a response (useful for mock transports)
pub fn encode_response(resp: &Response) -> Result<Vec<u8>> {
    let inner_len = 1 + resp.payload.len();
    let mut buf = Vec::with_capacity(1 + 2 + inner_len + 1 + 1);
    buf.push(frame::RSP_HEADER);
    buf.extend_from_slice(&(inner_len as u16).to_le_bytes());
    buf.push(resp.status);
    buf.extend_from_slice(&resp.payload);

    let crc_val = compute_crc(&buf[1..]);
    buf.push(crc_val);
    buf.push(frame::FOOTER);

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_encode_decode_roundtrip() {
        let cmd = Command { cmd_id: 0x01, payload: vec![42] };
        let encoded = encode_command(&cmd).unwrap();

        let resp = Response { status: Response::OK, payload: vec![1] };
        let encoded_resp = encode_response(&resp).unwrap();
        let decoded = decode_response(&encoded_resp).unwrap();

        assert_eq!(decoded.status, Response::OK);
        assert_eq!(decoded.payload, vec![1]);
    }

    #[test]
    fn crc_validation() {
        let resp = Response { status: 0, payload: vec![1, 2, 3] };
        let mut encoded = encode_response(&resp).unwrap();
        let crc_idx = encoded.len() - 2;
        encoded[crc_idx] ^= 0xFF;

        let result = decode_response(&encoded);
        assert!(matches!(result, Err(BridgeError::CrcMismatch { .. })));
    }

    #[test]
    fn invalid_header() {
        let data = vec![0xCC, 0x01, 0x00, 0x00, 0x00, 0x55];
        let result = decode_response(&data);
        assert!(matches!(result, Err(BridgeError::Frame(_))));
    }

    #[test]
    fn too_short() {
        let data = vec![0xBB, 0x01];
        let result = decode_response(&data);
        assert!(matches!(result, Err(BridgeError::Frame(_))));
    }

    #[test]
    fn invalid_footer() {
        let resp = Response { status: 0, payload: vec![] };
        let mut encoded = encode_response(&resp).unwrap();
        let footer_idx = encoded.len() - 1;
        encoded[footer_idx] = 0x00;

        let result = decode_response(&encoded);
        assert!(matches!(result, Err(BridgeError::Frame(_))));
    }

    #[test]
    fn empty_payload_roundtrip() {
        let resp = Response { status: 0x42, payload: vec![] };
        let encoded = encode_response(&resp).unwrap();
        let decoded = decode_response(&encoded).unwrap();
        assert_eq!(decoded.status, 0x42);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn large_payload_command() {
        let payload = vec![0xAB; 512];
        let cmd = Command { cmd_id: 0x03, payload: payload.clone() };
        let encoded = encode_command(&cmd).unwrap();
        assert_eq!(encoded[0], frame::CMD_HEADER);
        assert_eq!(*encoded.last().unwrap(), frame::FOOTER);
    }

    #[test]
    fn oversized_payload_rejected() {
        let payload = vec![0u8; frame::MAX_PAYLOAD + 1];
        let cmd = Command { cmd_id: 0x01, payload };
        let result = encode_command(&cmd);
        assert!(matches!(result, Err(BridgeError::Frame(_))));
    }
}
