extern crate bytes;
extern crate tokio_codec;

use bytes::{Bytes, BytesMut, IntoBuf, Buf, BufMut};
use tokio::codec::{Encoder, Decoder};
use std::convert::{TryFrom, TryInto, From, Into};

#[derive(Debug, Eq, PartialEq)]
pub enum FailReason {
    UnknownFail,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BrgMsg {
    ReqSession,
    ReqSessionReply(u64),
    SetSession(u64),
    SetSessionOk,
    SendData(Bytes),
    Fail(FailReason),
}

#[derive(Debug)]
pub enum BrgMsgParseError {
    InvalidOp(u8),
    CorruptedMessage,
}

use FailReason::*;

impl TryFrom<u8> for FailReason {
    type Error = ();
    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match code {
            0 => Ok(UnknownFail),
            _ => Err(()),
        }
    }
}

impl Into<u8> for &FailReason {
    fn into(self: Self) -> u8 {
        match self {
            UnknownFail => 0,
        }
    }
}

impl Into<u8> for FailReason {
    fn into(self: Self) -> u8 {
        (&self).into()
    }
}

use BrgMsg::*;
use BrgMsgParseError::*;

const U64_SIZE: usize = std::mem::size_of::<u64>();

impl TryFrom<&Bytes> for BrgMsg {
    type Error = BrgMsgParseError;
    fn try_from(src: &Bytes) -> Result<Self, Self::Error> {
        if src.len() == 0 {
            return Err(CorruptedMessage);
        }
        let op_code : u8 = src.get(0).unwrap().clone();
        match op_code {
            0 => if src.len() == 1 {
                Ok(ReqSession)
            } else {
                Err(CorruptedMessage)
            },
            1 => {
                let data = src.slice_from(1);
                if data.len() == U64_SIZE {
                    let mut buf = data.into_buf();
                    let id = buf.get_u64_be();
                    Ok(ReqSessionReply(id))
                } else {
                    Err(CorruptedMessage)
                }
            },
            2 => {
                let data = src.slice_from(1);
                if data.len() == U64_SIZE {
                    let mut buf = data.into_buf();
                    let id = buf.get_u64_be();
                    Ok(SetSession(id))
                } else {
                    Err(CorruptedMessage)
                }
            },
            3 => if src.len() == 1 {
                Ok(SetSessionOk)
            } else {
                Err(CorruptedMessage)
            },
            4 => {
                let data = src.slice_from(1);
                Ok(SendData(data))
            },
            5 => {
                let data = src.slice_from(1);
                if data.len() == U64_SIZE {
                    let mut buf = data.into_buf();
                    let err_code = buf.get_u8();
                    match FailReason::try_from(err_code) {
                        Ok(reason) => Ok(Fail(reason)),
                        _ => Err(CorruptedMessage),
                    }
                } else {
                    Err(CorruptedMessage)
                }
            },
            _ => Err(InvalidOp(op_code)),
        }
    }
}

impl Into<Bytes> for &BrgMsg {
    fn into(self: Self) -> Bytes {
        let (op_code, size): (u8, usize) = match self {
            ReqSession => (0, 1),
            ReqSessionReply(_) => (1, 1 + U64_SIZE),
            SetSession(_) => (2, 1 + U64_SIZE),
            SetSessionOk => (3, 1),
            SendData(d) => (4, 1 + d.len()),
            Fail(_) => (5, 2),
        };
        let mut bytes = BytesMut::with_capacity(size);
        bytes.put_u8(op_code);
        match self {
            ReqSessionReply(id) | SetSession(id) => bytes.put_u64_be(id.clone()),
            SendData(data) => bytes.put_slice(data),
            Fail(reason) => bytes.put_u8(reason.into()),
            _ => (),
        };
        Bytes::from(bytes)
    }
}

impl Into<Bytes> for BrgMsg {
    fn into(self: BrgMsg) -> Bytes {
        (&self).into()
    }
}

mod test {
    use std::convert::TryFrom;
    use bytes::{Bytes};
    use super::*;

    const REQ_SESSION_BYTES : [u8; 1] = [0];
    #[test]
    fn test_req_session_from_bytes() {
        let bytes = Bytes::from_static(&REQ_SESSION_BYTES);
        assert_eq!(BrgMsg::try_from(&bytes).unwrap(), ReqSession);
    }

    #[test]
    fn test_req_session_into_bytes() {
        let actual : Bytes = ReqSession.into();
        assert_eq!(REQ_SESSION_BYTES, *actual);
    }

    const REQ_SESSION_REPLY_BYTES : [u8; 9] = [1; 9];
    #[test]
    fn test_req_session_reply_from_bytes() {
        let bytes = Bytes::from_static(&REQ_SESSION_REPLY_BYTES);
        assert_eq!(BrgMsg::try_from(&bytes).unwrap(), ReqSessionReply(72340172838076673));
    }

    #[test]
    fn test_req_session_reply_into_bytes() {
        let actual : Bytes = ReqSessionReply(0x0101010101010101).into();
        assert_eq!(REQ_SESSION_REPLY_BYTES, *actual);
    }

    const SET_SESSION_BYTES : [u8; 9] = [2, 1, 1, 1, 1, 1 ,1, 1, 1];
    #[test]
    fn test_set_session_from_bytes() {
        let bytes = Bytes::from_static(&SET_SESSION_BYTES);
        assert_eq!(BrgMsg::try_from(&bytes).unwrap(), SetSession(0x0101010101010101));
    }

    #[test]
    fn test_set_session_into_bytes() {
        let actual : Bytes = SetSession(0x0101010101010101).into();
        assert_eq!(SET_SESSION_BYTES, *actual);
    }

    const SET_SESSION_OK_BYTES : [u8; 1] = [3];
    #[test]
    fn test_set_session_ok_from_bytes() {
        let bytes = Bytes::from_static(&SET_SESSION_OK_BYTES);
        assert_eq!(BrgMsg::try_from(&bytes).unwrap(), SetSessionOk);
    }

    #[test]
    fn test_set_session_ok_into_bytes() {
        let actual : Bytes = SetSessionOk.into();
        assert_eq!(SET_SESSION_OK_BYTES, *actual);
    }

    const SEND_DATA_BYTES : [u8; 3] = [4, 2, 2];
    const DATA_BYTES : [u8; 2] = [2, 2];
    #[test]
    fn test_send_data_from_bytes() {
        let bytes = Bytes::from_static(&SEND_DATA_BYTES);
        let bytes_data = Bytes::from_static(&DATA_BYTES);
        assert_eq!(BrgMsg::try_from(&bytes).unwrap(), SendData(bytes_data));
    }

    #[test]
    fn test_send_data_into_bytes() {
        let actual : Bytes = SendData((&DATA_BYTES as &[u8]).into()).into();
        assert_eq!(SEND_DATA_BYTES, *actual);
    }

    const SEND_DATA_EMPTY_BYTES : [u8; 1] = [4];
    const DATA_BYTES_EMPTY : [u8; 0] = [];
    #[test]
    fn test_send_data_empty_from_bytes() {
        let bytes = Bytes::from_static(&SEND_DATA_EMPTY_BYTES);
        let bytes_data = Bytes::from_static(&DATA_BYTES_EMPTY);
        assert_eq!(BrgMsg::try_from(&bytes).unwrap(), SendData(bytes_data));
    }

    #[test]
    fn test_send_data_empty_into_bytes() {
        let actual : Bytes = SendData((&DATA_BYTES_EMPTY as &[u8]).into()).into();
        println!("{:?}", SEND_DATA_EMPTY_BYTES);
        assert_eq!(SEND_DATA_EMPTY_BYTES, *actual);
    }

    #[test]
    fn test_fail_reason_from_u8() {
        assert_eq!(FailReason::try_from(0).unwrap(), UnknownFail);
    }

    #[test]
    fn test_fail_reason_into_u8() {
        let err : u8 = UnknownFail.into();
        assert_eq!(err, 0);
    }

    const FAIL_BYTES : [u8; 2] = [5, 0];
    #[test]
    fn test_fail_from_bytes() {
        let bytes = Bytes::from_static(&FAIL_BYTES);
        assert_eq!(BrgMsg::try_from(&bytes).unwrap(), Fail(UnknownFail));
    }
}