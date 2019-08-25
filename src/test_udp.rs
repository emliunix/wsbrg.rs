extern crate tokio_udp;
extern crate tokio_timer;

use tokio_udp::{UdpSocket, UdpFramed};
use tokio_codec::BytesCodec;
use std::net::SocketAddr;
use futures::prelude::*;
use futures::Future;
use futures::Stream;
use bytes::{Bytes, BytesMut, BufMut};
use tokio::timer::Interval;
use std::time::Duration;

use std::io::Error as IOError;
use std::io::ErrorKind as IOErrorKind;

fn blank_io_error() -> IOError {
    IOError::new(IOErrorKind::Other, "some error")
}

fn general_handler<T, E>(res: Result<T, E>) -> Result<(), std::io::Error> where E: std::fmt::Debug {
        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                use std::io::{Error, ErrorKind};
                Err(Error::new(ErrorKind::Other, format!("Some error: {:?}", err)))
            },
        }
}

fn test_msg() -> impl Future<Item=(), Error=()> {
    let from_addr = "0.0.0.0:8000".parse::<SocketAddr>().expect("invalid socketAddr");
    let to_addr = "0.0.0.0:8888".parse::<SocketAddr>().expect("invalid socketAddr");
    let udp_socket = UdpSocket::bind(&from_addr).expect("Failed to create UdpSocket");
    let mut msg_id = 0;
    UdpFramed::new(udp_socket, BytesCodec::new()).send_all(
        Interval::new_interval(Duration::from_secs(5))
        .map_err(|err| {
            eprintln!("Interval error: {:?}", err);
            blank_io_error()
        })
        .map(move |i| {
            msg_id += 1;
            println!("Sending test msg[{}], instant = {:?}", msg_id, i);
            let s: String = format!("a test msg, no. {}", msg_id);
            let mut buf = BytesMut::new();
            buf.put(s);
            (buf.freeze(), to_addr)
        })
    )
    .map(|_| ())
    .map_err(|err| eprintln!("Repeated test msg error: {:?}", err))
}

pub fn test() {
    let addr = "0.0.0.0:8888".parse::<SocketAddr>().expect("invalid socketAddr");
    let udpSocket =  UdpSocket::bind(&addr).expect("Failed to create UdpSocket");
    let udpStream = UdpFramed::new(udpSocket, BytesCodec::new());
    let (sink, istream) = udpStream.split();
    println!("Listening UDP MSG @ {}", addr);
    // tokio::run(sink.send_all(s).then(|res| {
    let t1 = sink.send_all(istream.map(|(b, srcAddr)| {
            let s : String = format!("FROM {:?} ECHO {:?}", srcAddr, b);
            println!("Recevied DGRAM {}", s);
            let mut buf = BytesMut::new();
            buf.put(s);
            (buf.freeze(), srcAddr)
        }))
        .map_err(|err| eprintln!("UDP processing failed: {:?}", err))
        .map(|_| ());
    let t2 = test_msg().map_err(|_| ());
    tokio::run(t1.join(t2).map(|_| ()).map_err(|_| ()));
}
