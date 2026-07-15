use std::{collections::HashMap, env, io::ErrorKind, sync::Arc};

use libserver::{dlog, executor, log};
use lune_abi::{
    DecodeErrorOrIo, ShmKey, StreamID,
    misc::SampleFormat,
    msg::{AllocatedShmObject, Request, Response, ResponseError, StreamSynced},
};
use safa_api::{
    abi::poll::{PollEntry, PollEvents},
    errors::ErrorStatus,
    shm::SharedObject,
    sockets::{UnixListener, UnixListenerBuilder, UnixSockConnection, UnixSockKind},
};

use crate::{
    com::ClientComPipe,
    stream::{self, AudioFormat},
};

fn handle_request(
    shared_objects: &mut HashMap<ShmKey, Arc<SharedObject>>,
    streams: &mut Vec<StreamID>,
    request: Request,
) -> Result<Response, ResponseError> {
    match request {
        Request::Ping(_) => Ok(Response::ok()),
        Request::AllocateObject(req) => {
            let size = req.size();

            let shm_obj = SharedObject::allocate(size)
                .expect("Audio server failed to allocate memory, crashing!1!!1!! :3");
            let shm_key = shm_obj.shm_key();

            shared_objects.insert(shm_key, Arc::new(shm_obj));
            Ok(Response::AllocatedObject(AllocatedShmObject::new(shm_key)))
        }
        Request::DestroyObject(key) => shared_objects
            .remove(&key)
            .ok_or(ResponseError::UnknownShmKey)
            .map(|_drop| Response::ok()),
        Request::CreateStream(req) => {
            let shm_obj = shared_objects
                .get(&req.region())
                .ok_or(ResponseError::UnknownShmKey)?;

            let samples_count = req.samples_count() as usize;
            if (shm_obj.data_ptr().len() / (req.bits_per_sample() as usize / 8)) < samples_count {
                return Err(ResponseError::ShmSizeTooSmall);
            }

            let sample_kind = SampleFormat::from_raw(req.sample_kind())
                .ok_or(ResponseError::InvalidSampleFormat)?;
            let audio_format = AudioFormat::from_raw(
                req.channels_count(),
                req.samples_per_second(),
                req.bits_per_sample(),
                sample_kind,
            )
            .ok_or(ResponseError::InvalidSampleFormat)?;

            stream::create_stream(shm_obj.clone(), audio_format, samples_count)
                .map_err(|()| ResponseError::ShmSizeTooSmall)
                .map(|stream_id| {
                    streams.push(stream_id);
                    Response::StreamID(stream_id)
                })
        }
        Request::SyncStream(req, stream_id) => {
            let offset = req.from_offset();
            let samples = req.samples_count();

            stream::sync_stream(stream_id, offset as usize, samples as usize)
                .map_err(|()| ResponseError::UnknownStreamID)
                .map(|n| Response::StreamSynced(StreamSynced::new(n as u32), stream_id))
        }
        Request::DestroyStream(stream_id) => {
            let found_idx = streams
                .iter()
                .position(|s| *s == stream_id)
                .ok_or(ResponseError::UnknownStreamID)?;

            streams.remove(found_idx);
            assert!(
                stream::remove_stream(stream_id),
                "Bad Stream ID wasn't found in mixer"
            );
            Ok(Response::ok())
        }
    }
}

async fn handle_connection_async(connection: UnixSockConnection) {
    dlog!("Handling a new connection");

    let mut stream_ids: Vec<StreamID> = Vec::with_capacity(1);
    let mut shared_objects: HashMap<ShmKey, Arc<SharedObject>> = HashMap::with_capacity(1);

    let pipe = Arc::new(ClientComPipe::new(connection));
    // No one else is going to be receiving requests and therefore we can take ownership of the receiver
    let mut receiver = pipe.receiver();
    loop {
        let request = match receiver.receive_request_async().await {
            Ok(r) => r,
            Err(ref e) => {
                if let DecodeErrorOrIo::Io(io) = e {
                    if io.kind() == ErrorKind::ConnectionAborted
                        || io.kind() == ErrorKind::ConnectionRefused
                        || io.kind() == ErrorKind::NotConnected
                        || io.kind() == ErrorKind::ConnectionReset
                    {
                        dlog!("Stream closed");
                        break;
                    } else if io.kind() == ErrorKind::WouldBlock {
                        continue;
                    } else {
                        dlog!("Error kind: {}", io.kind());
                    }
                }

                dlog!("Error: {e} from connection");
                continue;
            }
        };

        let response = handle_request(&mut shared_objects, &mut stream_ids, request)
            .unwrap_or_else(|e| Response::err(e));
        if let Err(e) = pipe.sender().send_response_async(response).await {
            dlog!("Error: {e} from connection, while sending for response");
            break;
        }
    }

    for stream_id in stream_ids {
        assert!(
            stream::remove_stream(stream_id),
            "leftover stream doesn't exist"
        );
    }
}

struct ListenerFuture<'a> {
    listener: &'a mut UnixListener,
}

impl<'a> ListenerFuture<'a> {
    pub fn new(listener: &'a mut UnixListener) -> Self {
        ListenerFuture { listener }
    }
}

impl<'a> Future for ListenerFuture<'a> {
    type Output = Result<UnixSockConnection, ErrorStatus>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.listener.accept() {
            Ok(o) => std::task::Poll::Ready(Ok(o)),
            Err(ErrorStatus::WouldBlock) => {
                executor::poll_resource(
                    PollEntry::new(self.listener.ri(), PollEvents::DATA_AVAILABLE),
                    cx.waker().clone(),
                );

                std::task::Poll::Pending
            }
            Err(e) => std::task::Poll::Ready(Err(e)),
        }
    }
}

async fn listener_loop(mut listener: UnixListener) {
    let opal_use_threads_var = env::var("OPAL_USE_THREADS")
        .map(|s| s.parse::<u8>().ok())
        .ok()
        .flatten()
        .unwrap_or(0);

    log!("Lune can use {} threads", opal_use_threads_var);
    let helper_threads_count =
        opal_use_threads_var.saturating_sub(2 /* this thread, and the mixer thread */);
    if helper_threads_count != 0 {
        for _ in 0..helper_threads_count {
            std::thread::spawn(|| executor::run(|| false));
        }
    }

    loop {
        dlog!("Listener Tick!");
        let sock_future = ListenerFuture::new(&mut listener);
        if let Ok(mut conn) = sock_future.await {
            dlog!("Got new connection!");
            conn.set_can_block(false)
                .expect("Failed to set blocking status");

            let future = handle_connection_async(conn);
            executor::spawn(future);
        }
    }
}

/// Listens for incoming connections and handles them
pub fn listener_thread() {
    let addr = lune_abi::CONNECT_ABSTRACT_ADDR;
    let mut listener_builder = UnixListenerBuilder::from_abstract_path(addr).unwrap();
    listener_builder
        .set_type(UnixSockKind::SeqPacket)
        .set_backlog(usize::MAX);
    listener_builder.set_non_blocking(true);

    let listener = listener_builder.bind().expect("Failed to bind a listener");
    log!("Lune Listening at {}", addr);
    executor::block_on(listener_loop(listener), || false);
}
