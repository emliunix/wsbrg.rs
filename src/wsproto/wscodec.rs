extern crate bytes;
extern crate tokio_codec;
extern crate ws;

use super::{Error, ErrorKind, Frame, OpCode};

use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use tokio::codec::{Decoder, Encoder};

pub struct WsCodec;

impl WsCodec {
    pub fn new() -> Self {
        WsCodec
    }
}

static MAX_PAYLOAD_LENGTH: u64 = 5 * (1 << 30);

impl Decoder for WsCodec {
    type Item = Frame;
    type Error = Error;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut buf = src.clone().into_buf();
        // at least header is ready
        if src.len() < 2 {
            return Ok(None);
        }
        // parse header
        let first: u8 = buf.get_u8();
        let second: u8 = buf.get_u8();

        // head
        let fin = first & 0x80 != 0;
        let rsv1 = first & 0x40 != 0;
        let rsv2 = first & 0x20 != 0;
        let rsv3 = first & 0x10 != 0;
        let opcode = OpCode::from(first & 0x0F);

        let masked = second & 0x80 != 0;
        let mut data_len: u64 = (second & 0x7F) as u64;

        // required_len
        let mut full_head_len: u64 = 2;
        if masked {
            full_head_len += 4
        };
        if data_len == 126 {
            full_head_len += 2
        } else if data_len == 127 {
            full_head_len += 8
        };

        if (src.len() as u64) < full_head_len {
            return Ok(None);
        }

        println!(
            "TRACE: fin = {}, rsv1 = {}, rsv2 = {}, rsv3 = {}, masked = {}, len1 = {}",
            fin, rsv1, rsv2, rsv3, masked, data_len
        );

        // parse len, mask bits
        data_len = match data_len {
            126 => buf.get_u16_be() as u64,
            127 => buf.get_u64_be(),
            _ => data_len,
        };

        let mask = if masked {
            let mut mask_buf: [u8; 4] = [0; 4];
            buf.take(4).copy_to_slice(&mut mask_buf);
            Some(mask_buf)
        } else {
            None
        };

        // check data length
        if data_len > MAX_PAYLOAD_LENGTH {
            return Err(Error::new(
                ErrorKind::Protocol,
                format!(
                    "Rejected frame with payload length exceeding defined max: {}.",
                    MAX_PAYLOAD_LENGTH
                ),
            ));
        }
        if src.len() < (full_head_len + data_len) as usize {
            return Ok(None);
        }

        // data
        src.split_to(full_head_len as usize);
        let mut data = src.split_to(data_len as usize);

        let frame = Frame {
            fin,
            rsv1,
            rsv2,
            rsv3,
            opcode,
            mask,
            payload: data,
        };
        Ok(Some(frame))
    }
}

impl Encoder for WsCodec {
    type Item = Frame;
    type Error = Error;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // head
        let x: u8 = item.opcode.into();
        let first: u8 = (if item.fin { 0x80u8 } else { 0x00u8 }) as u8
            | (if item.rsv1 { 0x40u8 } else { 0x00u8 }) as u8
            | (if item.rsv2 { 0x20u8 } else { 0x00u8 }) as u8
            | (if item.rsv3 { 0x10u8 } else { 0x00u8 }) as u8
            | x;
        dst.put_u8(first);
        // mask & payload_length
        let mask_bit: u8 = if item.mask.is_some() { 0x80 } else { 0x00 };
        match item.payload.len() {
            i if i < 126 => dst.put_u8(mask_bit | (i as u8)),
            i if i < 65536 => {
                dst.put_u8(mask_bit | 126u8);
                dst.put_u16_be(i as u16);
            }
            i => {
                dst.put_u8(mask_bit | 127u8);
                dst.put_u64_be(i as u64);
            }
        };
        if let Some(m) = item.mask {
            dst.put_slice(&m);
        }
        // payload
        dst.put_slice(&item.payload[..]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let bytes: [u8; 6] = [0x01, 0x01, 0x01, 0x00, 0x00, 0x00];

        let mut codec = WsCodec::new();
        let mut bytes = BytesMut::from(&bytes[..]);
        if let Ok(Some(v)) = codec.decode(&mut bytes) {
            assert_eq!(v.fin, false);
            assert_eq!(v.rsv1, false);
            assert_eq!(v.rsv2, false);
            assert_eq!(v.rsv2, false);
            assert_eq!(v.opcode, OpCode::Text);
            assert_eq!(v.mask, None);
            assert_eq!(v.payload, BytesMut::from(&b"\x01"[..]));
        } else {
            assert!(false);
        }
        assert_eq!(bytes.len(), 3);
    }

    #[test]
    fn test_not_ready() {
        let mut codec = WsCodec::new();
        if let Ok(v) = codec.decode(&mut BytesMut::from(&b"\x01"[..])) {
            assert!(v.is_none())
        } else {
            assert!(false);
        }
        if let Ok(v) = codec.decode(&mut BytesMut::from(&b"\x01\x01"[..])) {
            assert!(v.is_none())
        } else {
            assert!(false);
        }
    }
}
