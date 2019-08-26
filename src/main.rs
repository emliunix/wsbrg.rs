extern crate bytes;
extern crate tokio_tcp;
extern crate ws;

use futures::sink::Sink;
use futures::stream::Stream;
use futures::Future;
use std::net::SocketAddr;
use tokio::codec::{Decoder, Framed, FramedRead, LinesCodec};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;

mod wsproto;
// mod test;
// mod brg_session;
mod test_udp;

//mod ws_msg;
//mod some_codecs;

//use ws_msg::*;
//use BrgMsg::*;

//fn run() {
//    let addr = "localhost:8888".parse::<SocketAddr>().unwrap();
//    let listener = TcpListener::bind(&addr).unwrap();
//    let f = listener.incoming()
//        .for_each(|sock| {
//            let framed = Framed::new(sock, LinesCodec::new());
//            let a = tokio::spawn(framed.for_each(move |line| {
//                let b = tokio::spawn(framed.send(line).map(|_| {()}).map_err(|e| eprintln!("Send Error {:?}", e)));
//                Ok(())
//            }).map_err(|e| eprintln!("Read Error {:?}", e)));
//            Ok(())
//        })
//        .map_err(|e| {eprintln!("accept error {:?}", e)});
//    tokio::run(f);
//}
//
//extern crate httparse;
//
//use httparse::{Request, Result};
//use bytes::BytesMut;
//use tokio_tcp::TcpStream;
//
//#[derive(Debug)]
//struct ParseError(string);
//
//impl From<std::io::Error> for ParseError {
//    fn from(err: std::io::Error) -> Self {
//        ParseError(format!("IO ERROR: {:?}", err))
//    }
//}
//
//struct HttpReqParser;
//
//impl HttpReqParser {
//    fn new() -> Self {
//        HttpReqParser
//    }
//}
//
//impl Decoder for HttpReqParser {
//    type Item = Request;
//    type Error = ParseError;
//    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
//        let mut headers = [httparse::EMPTY_HEADER; 16];
//        let buf = src.as_byte_slice_mut();
//        let mut req = Request::new(&mut headers);
//        match self.req.parse(buf)? {
//            Status::Complete(s) => {
//                src.split_at(s);
//                Ok(Some(self.req))
//            },
//            Status::Partial => Ok(None)
//        }
//    }
//}
//
//use wsproto::WsCodec;
//
//enum WsSessionState<S> where S: AsyncRead + AsyncWrite {
//    HandShake(FramedRead<S, HttpReqParser>),
//    Work(Framed<S, WsCodec>),
//}
//
//struct WsSession<S> {
//    state: WsSessionState,
//}
//
//impl<S> WsSession<S> {
//    fn new(s: S) {
//        WsSession {
//            state: WsSessionState::HandShake(FramedRead::new(s, HttpReqParser)),
//        }
//    }
//}
//
//impl<S> Future for WsSession<S> {
//    type Item = ();
//    type Error = ParseError;
//    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//        match self.state {
//            WsSessionState::HandShake(f) => {
//                self.state = f.
//            }
//        }
//    }
//}
//
//fn run() {
//    let addr = "0.0.0.0:8888".parse::<SocketAddr>().unwrap();
//    let listener = TcpListener::bind(&addr).unwrap();
//    tokio::run(listener.incoming().for_each(|socket| {
//        let (reader, writer) = socket.split();
//        let http_reader = FramedRead::new(reader, HttpReqParser::new());
//        tokio::spawn(http_reader.for_each(move |i| {
//
//        }).then(move |res| {
//            match res {
//                Ok(_) => Ok(()),
//                Err(_) => Err(()),
//            }
//        }))
//    }))
//}

fn main() {
    //    run();
    // println!("Hello world!!!");
    // test::test();
    test_udp::test();
}
