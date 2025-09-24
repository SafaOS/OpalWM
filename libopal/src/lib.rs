use std::{
    io::{self, ErrorKind, Read, Write},
    ops::Deref,
    sync::{LazyLock, Mutex},
};

use opal_abi::com::{
    packet::MAX_PACKET_SIZE,
    request::{Request, RequestKind},
    response::{Response, ScreenInfo, event::WindowEvent},
};
use safa_api::{sockets::UnixSockConnection, syscalls::types::Ri};

pub mod icon;
pub mod window;

pub use opal_abi::com::response::event;
pub use opal_abi::com::response::event::Event;

static EVENTS_QUEUE: Mutex<Vec<WindowEvent>> = Mutex::new(Vec::new());

static WM_INFO: LazyLock<(Ri, Mutex<UnixSockConnection>)> = LazyLock::new(|| {
    use safa_api::sockets::{SockKind, UnixSockConnectionBuilder};
    let addr = opal_abi::CONNECT_ABSTRACT_ADDR;
    let mut builder = UnixSockConnectionBuilder::from_abstract_path(addr).unwrap();

    builder.set_type(SockKind::SeqPacket);
    builder
        .connect()
        .map(|k| (k.ri(), Mutex::new(k)))
        .unwrap_or_else(|_| panic!("Failed to establish connection with the Opal WM at {addr}"))
});

pub(crate) static WM_CONNECTION: LazyLock<&Mutex<UnixSockConnection>> =
    LazyLock::new(|| &WM_INFO.1);

pub fn connection_resource_id() -> Ri {
    WM_INFO.0
}

#[macro_export]
macro_rules! send_request {
    ($single: expr, $expected: ident $(( $capture: ident ))?) => {{
        let req = $single;
        let results = $crate::send_request_single(req).expect("failed to send request");
        match results {
            $crate::Response::Ok(o) => {
                match o {
                    ::opal_abi::com::response::OkResponse::$expected$(($capture))? => {
                        $($capture)?
                    }
                    o => panic!("Unexpected response: {o:#?}, expected: {} for request: {:#?}", stringify!($expected), req)
                }
            }
            $crate::Response::Err(e) => panic!("Error sending request: {e:#?}"),
            $crate::Response::Event(_) => unreachable!(),
        }
    }};
    ($req: expr, $expected: ident $(( $capture: ident ))?, $payload: expr) => {{
        let req = $req;
        let payload = $payload;
        let results = $crate::send_request_with_payload(req, payload).expect("failed to send request");
        match results {
            $crate::Response::Ok(o) => {
                match o {
                    ::opal_abi::com::response::OkResponse::$expected$(($capture))? => {
                        $($capture)?
                    }
                    o => panic!("Unexpected response: {o:#?}, expected: {} for request: {:#?}", stringify!($expected), req)
                }
            }
            $crate::Response::Err(e) => panic!("Error sending request: {e:#?}"),
            $crate::Response::Event(_) => unreachable!(),
        }
    }};

    ($req: expr, $expected: ident $(( $capture: ident ))?, then read $am: expr) => {{
        let req = $req;
        let mut connection = $crate::WM_CONNECTION
            .lock()
            .expect("Failed to acquire lock on the WM's connection");
        let results = $crate::send_request_single_inner(&mut connection, req).expect("failed to send request");
        match results {
            $crate::Response::Ok(o) => {
                match o {
                    ::opal_abi::com::response::OkResponse::$expected$(($capture))? => {
                        let amount = $am;
                        let mut bytes = vec![0; amount];
                        if let Err(e) = $crate::wm_read_bytes(&mut connection, &mut bytes) {
                            panic!("Failed to read {amount} bytes from the WM, error: {e:#?}, as per request: {req:#?}");
                        }
                        bytes
                    }
                    o => panic!("Unexpected response: {o:#?}, expected: {} for request: {:#?}", stringify!($expected), req)
                }
            }
            $crate::Response::Err(e) => panic!("Error sending request: {e:#?}"),
            $crate::Response::Event(_) => unreachable!(),
        }
    }};

}

pub(crate) fn wm_read_bytes(wm: &mut UnixSockConnection, bytes: &mut [u8]) -> io::Result<()> {
    wm.read_exact(bytes)
}

pub(crate) fn send_request_single(req: RequestKind) -> io::Result<Response> {
    let mut connection = WM_CONNECTION
        .lock()
        .expect("Failed to acquire lock on the WM's connection");
    send_request_single_inner(&mut connection, req)
}

pub(crate) fn send_request_single_inner(
    wm: &mut UnixSockConnection,
    req: RequestKind,
) -> io::Result<Response> {
    send_request(wm, req)?;
    read_response(wm)
}

pub(crate) fn send_request_with_payload(req: RequestKind, payload: &[u8]) -> io::Result<Response> {
    let mut connection = WM_CONNECTION
        .lock()
        .expect("Failed to acquire lock on the WM's connection");

    send_request(&mut connection, req)?;
    connection.write_all(payload)?;
    read_response(&mut connection)
}

#[inline]
fn read_response(wm: &mut UnixSockConnection) -> io::Result<Response> {
    let mut events = EVENTS_QUEUE
        .lock()
        .expect("Failed to acquire lock on events queue");

    let mut packet: [u8; MAX_PACKET_SIZE] = [0u8; MAX_PACKET_SIZE];
    let response = loop {
        let read = Read::read(&mut *wm, &mut packet)?;

        let msg = &packet[..read];

        let response = Response::decode(msg).expect("Couldn't Parse WM's response");
        match response {
            Response::Event(event) => {
                events.push(event);
            }
            other => break other,
        }
    };
    Ok(response)
}

#[inline]
fn send_request(wm: &mut UnixSockConnection, req: RequestKind) -> io::Result<()> {
    let request = Request::new(req);
    let (bytes, len) = request.encode();

    Write::write_all(&mut *wm, &bytes[..len])?;
    Ok(())
}

#[derive(Debug, Clone)]
/// Results of [`dequeue_events_blocking`], contains the events that were dequeued
pub enum DequeuedEvents {
    Single(WindowEvent),
    Multiple(Vec<WindowEvent>),
}

impl AsRef<[WindowEvent]> for DequeuedEvents {
    fn as_ref(&self) -> &[WindowEvent] {
        match self {
            DequeuedEvents::Single(event) => std::slice::from_ref(event),
            DequeuedEvents::Multiple(events) => events.as_ref(),
        }
    }
}

impl Deref for DequeuedEvents {
    type Target = [WindowEvent];
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
    let mut packet: [u8; MAX_PACKET_SIZE] = [0u8; MAX_PACKET_SIZE];
    let read = Read::read(&mut *wm, &mut packet)?;

    let msg = &packet[..read];

    let response = Response::decode(msg).expect("Couldn't Parse WM's response");
    match response {
        Response::Event(event) => {
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

    let mut packet: [u8; MAX_PACKET_SIZE] = [0u8; MAX_PACKET_SIZE];
    let read_results = Read::read(&mut *wm, &mut packet);
    wm.set_can_block(true).expect("Failed to enable blocking");

    let read = match read_results {
        Ok(0) => return Ok(None),
        Ok(amount) => amount,
        Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(None),
        Err(e) => return Err(e),
    };

    let msg = &packet[..read];

    let response = Response::decode(msg).expect("Couldn't Parse WM's response");
    match response {
        Response::Event(event) => {
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
    send_request!(RequestKind::Ping, Success)
}

static SCREEN_INFO: LazyLock<ScreenInfo> = LazyLock::new(|| {
    let info = send_request!(RequestKind::GetScreenInfo, ScreenInfo(i));
    info
});

/// Returns the (width, height) of the screen
pub fn get_screen_dimensions() -> (u32, u32) {
    (SCREEN_INFO.width, SCREEN_INFO.height)
}
