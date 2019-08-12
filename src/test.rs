extern crate hyper;
extern crate tokio;
extern crate tokio_tcp;
extern crate base64;
extern crate crypto;

use std::borrow::Borrow;
use std::net::SocketAddr;

use futures::{Future, Stream};
use futures::sink::{Sink};
use http::{HeaderValue, StatusCode};
use http::header::{UPGRADE, ORIGIN, SEC_WEBSOCKET_VERSION, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_ACCEPT, CONNECTION};
use httparse::Error::HeaderName;
use hyper::{Body, Request, Response, Server};
use hyper::server::Builder;
use hyper::upgrade::{OnUpgrade, Parts, Upgraded};
use tokio::net::TcpListener;
use tokio_codec::Framed;

use crate::wsproto::{WsCodec, Frame, OpCode};

use self::hyper::server::conn::Http;
use self::hyper::service::{service_fn_ok, service_fn, make_service_fn};
use self::crypto::digest::Digest;

fn process_ws_frame(mut msg: Frame) -> Option<Frame> {
    println!("Received message: {:?}", msg.opcode);
    // process mask if necessary
    msg.apply_mask();
    // print msg
    println!("Message: {:?}", msg.payload);
    // respond to ping
    let msg : Frame = Frame::ping();
    if msg.opcode == OpCode::Ping {
        Some(Frame::pong())
    } else {
        None
    }
}

fn ws_upgrade(req: Request<Body>) -> Response<Body> {
    // debug
    println!("All headers:");
    for (h, v) in req.headers().iter() {
        println!("{} => {}", h.as_str(), v.to_str().unwrap());
    }
    println!("End all headers");
    println!("Handling HTTP/WS Request...");
    let mut res = Response::new(Body::empty());
    // reject non websocket requests
    let mut is_valid = true;
    is_valid = is_valid && match req.headers().get(UPGRADE) {
        Some(v) => {
            if v == "websocket" {
                true
            } else {
                eprintln!("invalid upgrade header value: {}", v.to_str().unwrap());
                false
            }
        },
        None => {
            eprintln!("no upgrade header");
            false
        },
    };
    // this is for browser only
    // req.headers().contains_key(ORIGIN) &&
    is_valid = is_valid && match req.headers().get(SEC_WEBSOCKET_VERSION) {
        Some(v) => {
            if v == "13" {
                true
            } else {
                eprintln!("invalid sec-websocket-version: {}", v.to_str().unwrap());
                false
            }
        },
        None => {
            eprintln!("no sec-websocket-version header");
            false
        },
    };
    is_valid = is_valid && match req.headers().get(SEC_WEBSOCKET_KEY) {
        Some(v) => {
            match base64::decode(v.as_bytes()) {
                Ok(k) => {
                    if k.len() == 16 {
                        // Sec-WebSocket-Accept
                        let s = format!("{}{}", v.to_str().unwrap(), "258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
                        let mut sha1 = crypto::sha1::Sha1::new();
                        sha1.input_str(s.as_str());
                        let mut sha1_bytes = [0u8; 20];
                        sha1.result(&mut sha1_bytes);
                        let s2 = base64::encode(&sha1_bytes);
                        res.headers_mut().insert(SEC_WEBSOCKET_ACCEPT, HeaderValue::from_str(s2.as_str()).unwrap());
                        true
                    } else {
                        eprintln!("base64 decoded sec-websocket-key length mismatch: {}", k.len());
                        false
                    }
                },
                Err(e) => {
                    eprintln!("base64 decode sec-websocket-key error: {:?}", e);
                    false
                },
            }
        },
        None => {
            eprintln!("no sec-websocket-key header");
            false
        },
    };
    if !is_valid {
        eprintln!("Invalid request");
        *res.status_mut() = StatusCode::BAD_REQUEST;
        return res;
    }

    tokio::spawn(
        req.into_body().on_upgrade().then(move |r| {
        match r {
            Ok(upgraded) => {
                println!("HTTP Upgraded");
                let (sink, reader) = Framed::new(upgraded, WsCodec::new()).split();
                tokio::spawn(sink.send_all(reader.filter_map(process_ws_frame)).then(|_| Ok(()) ));
                Ok(())
            },
            Err(_) => Err(()),
        }
    }));
//    *sec-websocket-protocol,
//    *sec-websocket-extension,

    *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    res.headers_mut().insert(UPGRADE, HeaderValue::from_static("websocket"));
    res.headers_mut().insert(CONNECTION, HeaderValue::from_static("Upgrade"));
    // handling websocket headers
    res
}

pub fn test() {
    let addr = "0.0.0.0:8080".parse::<SocketAddr>().unwrap();
    let tcp = TcpListener::bind(&addr).unwrap();
    let server = Builder::new(
        tcp.incoming(), Http::new()
    ).serve(|| {
        service_fn_ok(ws_upgrade)
    });
    tokio::run(server.then(|res| {
        match res {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }))
    // service_fn_ok(service_fn(ws_upgrade)));
}