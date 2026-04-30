use std::io;
use std::sync::{LazyLock, Mutex};

use lune_abi::msg::{Message, Request, Response};
use safa_api::sockets::UnixSockConnection;

static CONNECTION: LazyLock<Mutex<UnixSockConnection>> = LazyLock::new(|| {
    use safa_api::sockets::{UnixSockConnectionBuilder, UnixSockKind};
    let addr = lune_abi::CONNECT_ABSTRACT_ADDR;
    let mut builder = UnixSockConnectionBuilder::from_abstract_path(addr).unwrap();

    builder.set_type(UnixSockKind::SeqPacket);
    builder
        .connect()
        .map(|k| Mutex::new(k))
        .unwrap_or_else(|e| {
            panic!("Failed to establish connection with the Opal WM at {addr}: {e:#?}")
        })
});

#[macro_export]
macro_rules! send_request_and_get {
    ($single: expr, $expected: ident ( $($capture: ident),+ )) => {{
        let req = $single;
        let results = $crate::server::send_request_single(req).expect("failed to send request");
        match results {
            ::lune_abi::msg::Response::$expected ( $($capture),+ ) => {
                    Ok({($($capture),+)})
            }
            ::lune_abi::msg::Response::Error(e) => Err(e),
            o => panic!("Unexpected response: {o:#?}, expected: {} for request: {:#?}", stringify!($expected), stringify!($single)),
        }
    }};
}

pub use crate::send_request_and_get;

#[macro_export]
macro_rules! send_request_or_panic {
    ($single: expr, $expected: ident $(( $capture: ident ))?) => {{
        match $crate::send_request_and_get!($single, $expected$(( $capture ))?) {
            Ok(value) => value,
            Err(e) => panic!("Unexpected error: {e:#?}"),
        }
    }};
}

pub use crate::send_request_or_panic;

pub(crate) fn send_request_single(req: Request) -> io::Result<Response> {
    let mut connection = CONNECTION
        .lock()
        .expect("Failed to acquire lock on the WM's connection");
    send_request_single_inner(&mut connection, req)
}

pub(crate) fn send_request_single_inner<'a>(
    wm: &mut UnixSockConnection,
    req: Request,
) -> io::Result<Response> {
    send_request(wm, req)?;
    read_response(wm)
}

#[inline]
fn read_response(server: &mut UnixSockConnection) -> io::Result<Response> {
    let response = {
        let (msg, _) = Message::decode_from(&mut *server).map_err(|d| match d {
            lune_abi::DecodeErrorOrIo::Io(io) => io,
            lune_abi::DecodeErrorOrIo::DecodeError(d) => {
                unreachable!("WM Responded with a decode error: {d:#?}")
            }
        })?;

        match msg {
            Message::Response(resp) => resp,
            Message::Request(_) => {
                unreachable!("No requests shall come before this")
            }
        }
    };
    Ok(response)
}

#[inline]
fn send_request(server: &mut UnixSockConnection, req: Request) -> io::Result<()> {
    let message = Message::Request(req);
    message.encode_into(server)?;
    Ok(())
}
