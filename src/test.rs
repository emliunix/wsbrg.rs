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
    if msg.opcode == OpCode::Text && msg.payload == "echo" {
        println!("Debug echo");
        return Some(Frame::text_bytes_mut(msg.payload))
    }
    if msg.opcode == OpCode::Ping {
        Some(Frame::pong())
    } else {
        None
    }
}

fn ws_gen_accept_header(v: &str) -> String {
    let s = format!("{}{}", v, "258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let mut sha1 = crypto::sha1::Sha1::new();
    sha1.input_str(s.as_str());
    let mut sha1_bytes = [0u8; 20];
    sha1.result(&mut sha1_bytes);
    base64::encode(&sha1_bytes)
}

enum HandshakeError {
    InvalidWSRequest,
}

fn ws_handshake(req: &Request<Body>) -> Result<Response<Body>, HandshakeError> {
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
                        res.headers_mut().insert(
                            SEC_WEBSOCKET_ACCEPT,
                            HeaderValue::from_str(
                                ws_gen_accept_header(v.to_str().unwrap()).as_str()
                            ).unwrap()
                        );
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
    if is_valid {
        //    *sec-websocket-protocol,
        //    *sec-websocket-extension,
        *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
        res.headers_mut().insert(UPGRADE, HeaderValue::from_static("websocket"));
        res.headers_mut().insert(CONNECTION, HeaderValue::from_static("Upgrade"));
        Ok(res)
    } else {
        Err(HandshakeError::InvalidWSRequest)
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
    match ws_handshake(&req) {
        Ok(res) => {
            tokio::spawn(
                req.into_body().on_upgrade().then(move |r| {
                match r {
                    Ok(upgraded) => {
                        println!("HTTP Upgraded");
                    //    let (sink, reader) = Framed::new(upgraded, WsCodec::new()).split();
                    //    tokio::spawn(sink.send_all(reader.filter_map(process_ws_frame)).then(|_| Ok(()) ));
                        process_upgraded(upgraded);
                        Ok(())
                    },
                    Err(_) => Err(()),
                }
            }));
            res
        },
        Err(_) => {
            eprintln!("Invalid request");
            let mut res = Response::new(Body::empty());
            *res.status_mut() = StatusCode::BAD_REQUEST;
            res
        }
    }

}

use futures::sync::mpsc;

fn process_upgraded(upgraded: Upgraded) {
    let (sink, reader) = Framed::new(upgraded, WsCodec::new()).split();
    let (sink_sender, sink_receiver) = mpsc::channel::<Frame>(64);
    my_spawn(reader.for_each(move |msg| {
        let sink_sender = (&sink_sender).clone();
        if let Some(f) = process_ws_frame(msg) {
            println!("Send back: {:?}", f);
            my_spawn(sink_sender.send(f));
        }
        Ok(())
    }));
    my_spawn(sink.send_all(sink_receiver.map_err(|_| ws::Error::new(ws::ErrorKind::Protocol, "some error"))));
}

use tokio::executor::Spawn;

fn my_spawn<T, E, F>(f: F) -> Spawn where E: std::fmt::Debug, F: Future<Item=T, Error=E> + 'static + Send {
    tokio::spawn(f.then(|res| {
        if let Err(e) = res {
            eprintln!("Error: {:?}", e)
        }
        Ok(())
    }))
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