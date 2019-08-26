extern crate bytes;

use bytes::{BigEndian, ByteOrder, BytesMut};

#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    /// Indicates a continuation frame of a fragmented message.
    Continue,
    /// Indicates a text data frame.
    Text,
    /// Indicates a binary data frame.
    Binary,
    /// Indicates a close control frame.
    Close,
    /// Indicates a ping control frame.
    Ping,
    /// Indicates a pong control frame.
    Pong,
    /// Indicates an invalid opcode was received.
    Bad(u8),
}

impl From<u8> for OpCode {
    fn from(v: u8) -> Self {
        use OpCode::*;
        match v {
            0 => Continue,
            1 => Text,
            2 => Binary,
            8 => Close,
            9 => Ping,
            10 => Pong,
            v => Bad(v),
        }
    }
}

impl Into<u8> for OpCode {
    fn into(self: Self) -> u8 {
        use OpCode::*;
        match self {
            Continue => 0,
            Text => 1,
            Binary => 2,
            Close => 8,
            Ping => 9,
            Pong => 10,
            Bad(v) => v,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub fin: bool,
    pub rsv1: bool,
    pub rsv2: bool,
    pub rsv3: bool,
    pub opcode: OpCode,
    pub mask: Option<[u8; 4]>,
    pub payload: BytesMut,
}

impl Default for Frame {
    fn default() -> Frame {
        Frame {
            fin: true,
            rsv1: false,
            rsv2: false,
            rsv3: false,
            opcode: OpCode::Bad(11),
            mask: None,
            payload: BytesMut::with_capacity(0),
        }
    }
}

impl Frame {
    pub fn apply_mask(&mut self) -> bool {
        match self.mask {
            None => false,
            Some(m) => {
                let mu32 = BigEndian::read_u32(&m);
                for i in 0..(self.payload.len() / 4) {
                    let r = (i * 4)..((i + 1) * 4);
                    let v = BigEndian::read_u32(&self.payload[r.clone()]);
                    let v = mu32 ^ v;
                    BigEndian::write_u32(&mut self.payload[r.clone()], v);
                }
                for i in (self.payload.len() & 0xFFFC)..self.payload.len() {
                    let mu8 = m[i & 0x03];
                    self.payload[i] ^= mu8;
                }
                true
            }
        }
    }

    pub fn pong() -> Frame {
        Self::pong_bytes_mut(BytesMut::with_capacity(0))
    }

    pub fn pong_bytes_mut(b: BytesMut) -> Frame {
        let mut frame = Frame::default();
        frame.opcode = OpCode::Pong;
        frame.payload = b;
        frame
    }

    pub fn ping() -> Frame {
        let mut frame = Frame::default();
        frame.opcode = OpCode::Ping;
        frame
    }

    pub fn text(s: &str) -> Frame {
        Self::text_bytes_mut(BytesMut::from(s))
    }

    pub fn text_bytes_mut(b: BytesMut) -> Frame {
        let mut frame = Frame::default();
        frame.opcode = OpCode::Text;
        frame.mask = None;
        frame.payload = b;
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_eq() {
        assert_eq!(OpCode::Pong, OpCode::Pong);
        assert_ne!(OpCode::Ping, OpCode::Pong);
    }
}
