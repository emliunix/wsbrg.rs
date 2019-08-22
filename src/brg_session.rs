extern crate tokio_udp;

use std::collections::BTreeMap;
use futures::prelude::*;
use bytes::BytesMut;
use tokio_udp::UdpSocket;

pub enum BrgConnectionError {
    SomeError(String),
}

pub trait BrgConnection :
    Stream<Item=BytesMut, Error=BrgConnectionError> +
    Sink<SinkItem=BytesMut, SinkError=BrgConnectionError>
{

}

struct BrgSession<C> where C: BrgConnection {
    next_id: u32,
    conns: BTreeMap<u32, C>,
}

struct UDPConnection {
    conn_id: u32,
}

impl Stream for UDPConnection {
    type Item = BytesMut;
    type Error = BrgConnectionError;

}

impl Sink for UDPConnection {
    type SinkItem = BytesMut;
    type SinkError = BrgConnectionError;
}

impl BrgConnection for UDPConnection {

}

enum UDPConnectionState {
    MissingConfig,
    Open(UdpSocket),
    Closed,
}

fn udp_test() {
}