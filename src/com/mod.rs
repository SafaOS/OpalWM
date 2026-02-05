use std::{
    cell::UnsafeCell,
    io::{self, ErrorKind, Read, Write},
    sync::{Mutex, MutexGuard},
};

use libopal::OpalV1;
use opal_abi::{
    DecodeErrorOrIo,
    msg::{Message, Request, Response},
};
use safa_api::{
    abi::poll::{PollEntry, PollEvents},
    sockets::UnixSockConnection,
    syscalls::types::Ri,
};

use crate::executor;

pub mod listener;

/// Represents a future for sending a response over a client communication channel.
pub struct ComSendFuture<'a, 'b> {
    sender: &'a mut ClientComSender<'b>,
    message: Message,
}

impl<'a, 'b> Future for ComSendFuture<'a, 'b> {
    type Output = Result<(), io::Error>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        let sender = &mut this.sender;
        let message = &this.message;

        match sender.send_message_raw(message) {
            Ok(()) => std::task::Poll::Ready(Ok(())),
            Err(err) => match err.kind() {
                ErrorKind::WouldBlock => {
                    let ri = this.sender.pipe.ri();
                    let waker = cx.waker().clone();
                    executor::poll_resource(PollEntry::new(ri, PollEvents::CAN_WRITE), waker);
                    std::task::Poll::Pending
                }
                _ => std::task::Poll::Ready(Err(err)),
            },
        }
    }
}

/// A future for receiving a response from the client.
pub struct ComReceiveFuture<'a, 'b> {
    receiver: &'a mut ClientComReceiver<'b>,
}

impl<'a, 'b> Future for ComReceiveFuture<'a, 'b> {
    type Output = Result<Request, ReadError>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.receiver.receive_request() {
            Ok(req) => std::task::Poll::Ready(Ok(req)),
            Err(ReadError::DecodeError(p)) => {
                std::task::Poll::Ready(Err(ReadError::DecodeError(p)))
            }
            Err(ReadError::Io(err)) => match err.kind() {
                ErrorKind::WouldBlock => {
                    let ri = self.receiver.pipe.ri();
                    let waker = cx.waker().clone();
                    executor::poll_resource(PollEntry::new(ri, PollEvents::DATA_AVAILABLE), waker);
                    std::task::Poll::Pending
                }
                _ => std::task::Poll::Ready(Err(ReadError::Io(err))),
            },
        }
    }
}

/// A lock guard for the Sender part of the [`ClientComPipe`]
///
/// FIXME: Ensure that this is used once per thread, as so no 2 async tasks can use a receiver at the same time, because this holds a mutex guard so it isn't Send.
pub struct ClientComSender<'a> {
    _guard: MutexGuard<'a, ()>,
    pipe: &'a ClientComPipe,
}

/// FIXME: These aren't really send thanks to Mutex guard
/// but currently they aren't used in a context within the same executor sooo...
unsafe impl<'a> Send for ClientComSender<'a> {}

impl<'a> ClientComSender<'a> {
    /// Sends a response to the client, returns [`WouldBlock`] if the pipe is not ready to send.
    pub fn send_message_raw(&mut self, message: &Message) -> Result<(), io::Error> {
        message.encode_into(self).map(|_| ())
    }

    /// Sends a response to the client, blocks until the response is sent.
    pub async fn send_response_async(&mut self, response: Response) -> Result<(), io::Error> {
        ComSendFuture {
            message: Message::new_response(response),
            sender: self,
        }
        .await
    }
}

/// A lock guard for the Receiver part of the [`ClientComPipe`].
///
/// FIXME: Ensure that this is used once per thread, as so no 2 async tasks can use a receiver at the same time, because this holds a mutex guard so it isn't Send.
pub struct ClientComReceiver<'a> {
    _guard: MutexGuard<'a, ()>,
    pipe: &'a ClientComPipe,
}

unsafe impl<'a> Send for ClientComReceiver<'a> {}

impl<'a> ClientComReceiver<'a> {
    /// Receives a request from the client, blocks until the request is received.
    fn receive_request(&mut self) -> Result<Request, ReadError> {
        let (this, _) = Message::decode_from(self)?;
        match this {
            Message::OpalV1(OpalV1::Request(request)) => Ok(request),
            _ => Err(ReadError::DecodeError(
                opal_abi::DecodeError::UnexpectedMessage,
            )),
        }
    }

    /// Receives a request from the client, blocks until the request is received.
    pub async fn receive_request_async(&mut self) -> Result<Request, ReadError> {
        ComReceiveFuture { receiver: self }.await
    }
}

/// A Wrapper over a bi-directonal communication pipe, that can send data to and from the client.
///
/// This structure allows you to separate read and write operations on the client giving different locks for send and receive operations,
/// obviously this means that there is no guarantee that the client will receive the response in the same order as the request was sent, but allows to send events to the client.
pub struct ClientComPipe {
    sender_lock: Mutex<()>,
    receiver_lock: Mutex<()>,
    connection: UnsafeCell<UnixSockConnection>,
}

/// An Error that happened during reading a request from a Client
pub type ReadError = DecodeErrorOrIo;

unsafe impl Send for ClientComPipe {}
unsafe impl Sync for ClientComPipe {}

impl ClientComPipe {
    pub const fn new(inner: UnixSockConnection) -> Self {
        Self {
            sender_lock: Mutex::new(()),
            receiver_lock: Mutex::new(()),
            connection: UnsafeCell::new(inner),
        }
    }

    /// Acquires lock on a sender that can be used to send responses to the client.
    pub fn sender<'a>(&'a self) -> ClientComSender<'a> {
        ClientComSender {
            _guard: self
                .sender_lock
                .lock()
                .expect("Failed to acquire lock on the sending side of a communication pipe"),
            pipe: self,
        }
    }

    /// Acquires lock on a receiver that can be used to receive requests from the client.
    pub fn receiver<'a>(&'a self) -> ClientComReceiver<'a> {
        ClientComReceiver {
            _guard: self
                .receiver_lock
                .lock()
                .expect("Failed to acquire lock on the receiving side of a communication pipe"),
            pipe: self,
        }
    }

    /// Returns a resource ID (RI) associated with the communication pipe.
    pub const fn ri(&self) -> Ri {
        unsafe { &*self.connection.get() }.ri()
    }
}

impl<'a> Read for ClientComReceiver<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe { Read::read(&mut *self.pipe.connection.get(), buf) }
    }
}

impl<'a> Write for ClientComSender<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe { Write::write(&mut *self.pipe.connection.get(), buf) }
    }
    fn flush(&mut self) -> io::Result<()> {
        unsafe { &mut *self.pipe.connection.get() }.flush()
    }
}
