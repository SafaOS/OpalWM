use std::{
    io::{self, ErrorKind},
    ops::Deref,
    sync::{LazyLock, Mutex},
};

use opal_abi::{
    DecodeError, DecodeErrorOrIo,
    msg::{
        GetScreenInfo, Message, Ping,
        event::Event,
        request::Request,
        response::{Response, ScreenInfo},
    },
};
use safa_api::{sockets::UnixSockConnection, syscalls::types::Ri};

pub mod icon;
pub mod keys;
pub mod shm;
pub mod window;

pub use opal_abi::defs;
pub use opal_abi::display;
pub use opal_abi::msg::OpalV1;
pub use opal_abi::msg::event;
pub use opal_abi::msg::event::WindowEvent;

static EVENTS_QUEUE: Mutex<Vec<Event>> = Mutex::new(Vec::new());

static WM_INFO: LazyLock<(Ri, Mutex<UnixSockConnection>)> = LazyLock::new(|| {
    use safa_api::sockets::{UnixSockConnectionBuilder, UnixSockKind};
    let addr = opal_abi::CONNECT_ABSTRACT_ADDR;
    let mut builder = UnixSockConnectionBuilder::from_abstract_path(addr).unwrap();

    builder.set_type(UnixSockKind::SeqPacket);
    builder
        .connect()
        .map(|k| (k.ri(), Mutex::new(k)))
        .unwrap_or_else(|e| {
            panic!("Failed to establish connection with the Opal WM at {addr}: {e:#?}")
        })
});

pub(crate) static WM_CONNECTION: LazyLock<&Mutex<UnixSockConnection>> =
    LazyLock::new(|| &WM_INFO.1);

pub fn connection_resource_id() -> Ri {
    WM_INFO.0
}

#[macro_export]
macro_rules! send_request_and_get {
    ($single: expr, $expected: ident $(( $capture: ident ))?) => {{
        let req = $single;
        let results = $crate::send_request_single(req).expect("failed to send request");
        match results {
            ::opal_abi::msg::response::Response::$expected$(($capture))? => {
                    Ok({$($capture)?})
            }
            ::opal_abi::msg::response::Response::Error(e) => Err(e),
            o => panic!("Unexpected response: {o:#?}, expected: {} for request: {:#?}", stringify!($expected), stringify!($single)),
        }
    }};

    // ($req: expr, $expected: ident $(( $capture: ident ))?, $payload: expr) => {{
    //     let req = $req;
    //     let payload = $payload;
    //     let results = $crate::send_request_with_payload(req, payload).expect("failed to send request");
    //     match results {
    //         $crate::Response::Ok(o) => {
    //             match o {
    //                 ::opal_abi::com::response::OkResponse::$expected$(($capture))? => {
    //                     Ok({$($capture)?})
    //                 }
    //                 o => panic!("Unexpected response: {o:#?}, expected: {} for request: {:#?}", stringify!($expected), req)
    //             }
    //         }
    //         $crate::Response::Err(e) => Err(e),
    //         $crate::Response::Event(_) => unreachable!(),
    //     }
    // }};


    // ($req: expr, $expected: ident $(( $capture: ident ))?, then read $am: expr) => {{
    //     let req = $req;
    //     let mut connection = $crate::WM_CONNECTION
    //         .lock()
    //         .expect("Failed to acquire lock on the WM's connection");
    //     let results = $crate::send_request_single_inner(&mut connection, req).expect("failed to send request");
    //     match results {
    //         $crate::Response::Ok(o) => {
    //             match o {
    //                 ::opal_abi::com::response::OkResponse::$expected$(($capture))? => {
    //                     let amount = $am;
    //                     let mut bytes = vec![0; amount];
    //                     if let Err(e) = $crate::wm_read_bytes(&mut connection, &mut bytes) {
    //                         panic!("Failed to read {amount} bytes from the WM, error: {e:#?}, as per request: {req:#?}");
    //                     }
    //                     Ok(bytes)
    //                 }
    //                 o => panic!("Unexpected response: {o:#?}, expected: {} for request: {:#?}", stringify!($expected), req)
    //             }
    //         }
    //         $crate::Response::Err(e) => Err(e),
    //         $crate::Response::Event(_) => unreachable!(),
    //     }
    // }};
}

#[macro_export]
macro_rules! send_request_or_panic {
    ($single: expr, $expected: ident $(( $capture: ident ))?) => {{
        match $crate::send_request_and_get!($single, $expected$(( $capture ))?) {
            Ok(value) => value,
            Err(e) => panic!("Unexpected error: {e:#?}"),
        }
    }};
}

pub(crate) fn send_request_single(req: Request) -> io::Result<Response> {
    let mut connection = WM_CONNECTION
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

// pub(crate) fn send_request_with_payload(req: RequestKind, payload: &[u8]) -> io::Result<Response> {
//     let mut connection = WM_CONNECTION
//         .lock()
//         .expect("Failed to acquire lock on the WM's connection");

//     send_request(&mut connection, req)?;
//     connection.write_all(payload)?;
//     read_response(&mut connection)
// }

#[inline]
fn read_response(wm: &mut UnixSockConnection) -> io::Result<Response> {
    let mut events = EVENTS_QUEUE
        .lock()
        .expect("Failed to acquire lock on events queue");

    let response = loop {
        let (msg, _) = Message::decode_from(&mut *wm).map_err(|d| match d {
            opal_abi::DecodeErrorOrIo::Io(io) => io,
            opal_abi::DecodeErrorOrIo::DecodeError(d) => {
                unreachable!("WM Responded with a decode error: {d:#?}")
            }
        })?;

        match msg {
            Message::OpalV1(OpalV1::Event(event)) => {
                events.push(event);
            }
            Message::OpalV1(OpalV1::Response(resp)) => break resp,
            Message::OpalV1(OpalV1::Request(_)) => {
                unreachable!("No requests shall come before this")
            }
        }
    };
    Ok(response)
}

#[inline]
fn send_request(wm: &mut UnixSockConnection, req: Request) -> io::Result<()> {
    let message = Message::new_request(req);
    message.encode_into(wm)?;
    Ok(())
}

#[derive(Debug, Clone)]
/// Results of [`dequeue_events_blocking`], contains the events that were dequeued
pub enum DequeuedEvents {
    Single(Event),
    Multiple(Vec<Event>),
}

impl AsRef<[Event]> for DequeuedEvents {
    fn as_ref(&self) -> &[Event] {
        match self {
            DequeuedEvents::Single(event) => std::slice::from_ref(event),
            DequeuedEvents::Multiple(events) => events.as_ref(),
        }
    }
}

impl Deref for DequeuedEvents {
    type Target = [Event];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/// Blockingly wait for an event from the window manager or dequeue waiting unhandled events.
///
/// The non-blocking equalivent would be [`dequeue_events_non_blocking`].
pub fn dequeue_events_blocking() -> io::Result<DequeuedEvents> {
    {
        let mut events = EVENTS_QUEUE
            .lock()
            .expect("Failed to acquire lock on events queue");

        let events = std::mem::take(&mut *events);
        if !events.is_empty() {
            return Ok(DequeuedEvents::Multiple(events));
        }
    }

    let mut wm = WM_CONNECTION.lock().expect("Failed to lock WM connection");

    let (message, _) = Message::decode_from(&mut *wm).map_err(|e| match e {
        DecodeErrorOrIo::Io(io) => io,
        DecodeErrorOrIo::DecodeError(d) => unreachable!("Couldn't parse WM's response: {d:#?}"),
    })?;

    match message {
        Message::OpalV1(OpalV1::Event(event)) => {
            return Ok(DequeuedEvents::Single(event));
        }
        other => unreachable!(
            "Shouldn't get any other kind of responses while waiting for events, got: {other:#?}"
        ),
    }
}

/// Attempt to dequeue Events from the WM connection pipe, returns Ok(None) if no events are available currently instead of blocking until there is.
///
/// The blocking equalivent would be [`dequeue_events_blocking`].
pub fn dequeue_events_non_blocking() -> io::Result<Option<DequeuedEvents>> {
    {
        let mut events = EVENTS_QUEUE
            .lock()
            .expect("Failed to acquire lock on events queue");

        let events = std::mem::take(&mut *events);
        if !events.is_empty() {
            return Ok(Some(DequeuedEvents::Multiple(events)));
        }
    }

    let mut wm = WM_CONNECTION.lock().expect("Failed to lock WM connection");

    wm.set_can_block(false).expect("Failed to disable blocking");
    let results = Message::decode_from(&mut *wm);
    wm.set_can_block(true).expect("Failed to enable blocking");

    // TODO: decode_from_buf and MAX_MESSAGE_SIZE...
    let message = match results {
        Err(DecodeErrorOrIo::DecodeError(de)) => match de {
            DecodeError::BufferTooSmall => return Ok(None),
            de => unreachable!("Unexpected decode error: {de:#?}"),
        },
        Ok((message, _)) => message,
        Err(DecodeErrorOrIo::Io(io)) if io.kind() == ErrorKind::WouldBlock => return Ok(None),
        Err(DecodeErrorOrIo::Io(io)) => return Err(io),
    };

    match message {
        Message::OpalV1(OpalV1::Event(event)) => {
            return Ok(Some(DequeuedEvents::Single(event)));
        }
        other => unreachable!(
            "Shouldn't get any other kind of responses while waiting for events, got: {other:#?}"
        ),
    }
}

pub use safa_api;
pub use safa_api::abi as safa_abi;

/// Polls the WM for events, returning the events if any are available, also polls additional user-supplied resources
/// given in the `entries` slice, the first of the slice is going to be reused for WM's resource, and therefore the len must be at least 1.
pub fn dequeue_events_and_poll(
    entries: &mut [safa_abi::poll::PollEntry],
) -> io::Result<Option<DequeuedEvents>> {
    assert!(
        !entries.is_empty(),
        "First entry is going to be reused for WM"
    );

    entries[0] = safa_abi::poll::PollEntry::new(
        connection_resource_id(),
        safa_abi::poll::PollEvents::DATA_AVAILABLE,
    );
    safa_api::syscalls::io::poll_resources(entries, None)
        .map_err(|err| safa_api::errors::into_io_error(err))?;

    if entries[0]
        .returned_events()
        .contains(safa_abi::poll::PollEvents::DISCONNECTED)
    {
        std::process::exit(0);
    }

    if entries[0]
        .returned_events()
        .contains(safa_abi::poll::PollEvents::DATA_AVAILABLE)
    {
        dequeue_events_non_blocking()
    } else {
        Ok(None)
    }
}

/// Initializes the client that is going to communicate with the WM
/// Panicks on failure
pub fn init() {
    send_request_or_panic!(Request::Ping(Ping), Success(s));
}

static SCREEN_INFO: LazyLock<ScreenInfo> = LazyLock::new(|| {
    let info = send_request_or_panic!(Request::GetScreenInfo(GetScreenInfo), ScreenInfo(i));
    info
});

/// Returns the (width, height) of the screen
pub fn get_screen_dimensions() -> (u32, u32) {
    (SCREEN_INFO.width, SCREEN_INFO.height)
}
