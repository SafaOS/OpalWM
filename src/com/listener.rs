use std::{
    collections::HashMap,
    io::ErrorKind,
    process::{Command, Stdio},
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use libopal::{defs::ShmKey, window::WindowFlags};
use opal_abi::{
    Name,
    msg::{
        CreateWindow, DestroyObject, FocusWindow, GetScreenInfo, GetWindowInfo, IconLoaded,
        LoadIcon, NewSharedObject, Ping, PreloadIcon,
        request::Request,
        response::{IconPreloaded, Response, ResponseError, ScreenInfo, WindowCreated, WindowInfo},
    },
};
use safa_api::{
    abi::poll::{PollEntry, PollEvents},
    errors::ErrorStatus,
    shm::SharedObject,
    sockets::{UnixListener, UnixListenerBuilder, UnixSockConnection, UnixSockKind},
};

use crate::{
    com::{ClientComPipe, ClientComReceiver, ClientComSender, ReadError},
    framebuffer::{FB_INFO, Pixel},
    log, logging,
    window::{self, WINDOWS, WinID, Window, WindowKind},
};
use libserver::executor;
use libserver::{dlog, elog, wlog};

fn spawn_hello() {
    if let Err(err) = Command::new("sys:/bin/hello_world")
        .stdout(Stdio::from(logging::console_clone()))
        .stderr(Stdio::from(logging::console_clone()))
        .stdin(Stdio::from(logging::console_clone()))
        .spawn()
    {
        elog!("Failed to spawn hello_world process: {}", err);
    }
}

pub fn spawn_terminal() {
    if let Err(err) = Command::new("sys:/bin/terminal")
        .stdout(Stdio::from(logging::console_clone()))
        .stderr(Stdio::from(logging::console_clone()))
        .stdin(Stdio::from(logging::console_clone()))
        .spawn()
    {
        elog!("Failed to spawn terminal process: {}", err);
    }
}

fn spawn_desktop() {
    if let Err(err) = Command::new("sys:/bin/desktop")
        .stdout(Stdio::from(logging::console_clone()))
        .stderr(Stdio::from(logging::console_clone()))
        .stdin(Stdio::from(logging::console_clone()))
        .spawn()
    {
        elog!("Failed to spawn desktop process: {}", err);
    }
}

async fn write_response(sender: &mut ClientComSender<'_>, response: Response) -> Result<(), ()> {
    if let Err(e) = sender.send_response_async(response).await {
        elog!("Error writing to socket '{e}', disconnecting...");
        return Err(());
    }
    Ok(())
}

fn handle_create_window(
    pipe: &Arc<ClientComPipe>,
    window_ids: &mut Vec<WinID>,
    shm_objects: &HashMap<ShmKey, Arc<SharedObject>>,
    request: CreateWindow,
) -> Result<Response, ResponseError> {
    dlog!("Create window: {shm_objects:#?}, request: {request:#?}");
    let shm_key = request.shm_key();
    let shm_object = shm_objects
        .get(&shm_key)
        .ok_or(ResponseError::InvalidShmKey)?
        .clone();
    let name = *request.name_inner();
    let height = request.height() as usize;
    let width = request.width() as usize;
    let flags = request.flags();

    let abs_pos = request.x().is_some() || request.y().is_some();

    let window = Window::new_filled_with(
        name,
        request.icon(),
        request.x().map(|x| x as isize).unwrap_or(0),
        request.y().map(|y| y as isize).unwrap_or(0),
        width,
        height,
        Pixel::NONE,
        flags,
        Some((shm_object, pipe.clone())),
    )
    .ok_or(ResponseError::SharedObjectTooSmall)?;

    let mut kind = WindowKind::Normal;
    if flags.contains(WindowFlags::BG_WINDOW) {
        kind = WindowKind::Background;
    } else if flags.contains(WindowFlags::OVERLAY_WINDOW) {
        kind = WindowKind::Overlay;
    }

    window::add_window(window, kind, !abs_pos)
        .map(|id| {
            dlog!("Added Window {id}, with the SHM Key {shm_key} for a client");
            window_ids.push(id);
            WindowCreated::new(id)
        })
        .map(Response::WindowCreated)
        .ok_or(ResponseError::OtherFatalError)
}

fn handle_preload_icon(
    shm_objects: &HashMap<ShmKey, Arc<SharedObject>>,
    request: PreloadIcon,
) -> Result<Response, ResponseError> {
    let size = request.load_bytes();
    let key = request.load_from();
    if size > (256 * 256 * size_of::<Pixel>()) + 128 {
        return Err(ResponseError::TooLarge);
    }

    let src_obj = shm_objects.get(&key).ok_or(ResponseError::InvalidShmKey)?;

    let src_obj_data = unsafe { src_obj.data() };
    if src_obj_data.len() < size {
        return Err(ResponseError::SharedObjectTooSmall);
    }

    // TODO: Validate the icon data
    let icon_data = src_obj_data[..size].to_vec();
    let id = crate::icons::add_icon(icon_data);
    Ok(Response::IconPreloaded(IconPreloaded::new(id)))
}

fn handle_load_icon(
    shm_objects: &HashMap<ShmKey, Arc<SharedObject>>,
    request: LoadIcon,
    icon_data: &[u8],
) -> Result<Response, ResponseError> {
    let shm_key = request.store_into();
    let object = shm_objects
        .get(&shm_key)
        .ok_or(ResponseError::InvalidShmKey)?;

    // Safety: Reading and muttating objects is done with synchronization and checks.
    let dest_obj_data = unsafe { object.data_ptr().as_mut() };
    if dest_obj_data.len() < icon_data.len() {
        return Err(ResponseError::SharedObjectTooSmall);
    }

    let dest = &mut dest_obj_data[..icon_data.len()];
    dest.copy_from_slice(icon_data);
    Ok(Response::LoadedIcon(IconLoaded::new(dest.len())))
}

async fn handle_event_on(
    window_ids: &mut Vec<WinID>,
    shm_objects: &mut HashMap<ShmKey, Arc<SharedObject>>,
    pipe: &Arc<ClientComPipe>,
    receiver: &mut ClientComReceiver<'_>,
) -> bool {
    let response = match receiver.receive_request_async().await {
        Ok(req) => match req {
            Request::AllocateObject(request) => SharedObject::allocate(request.size())
                .map_err(|_| ResponseError::OtherFatalError)
                .map(|obj| {
                    let key = obj.shm_key();
                    shm_objects.insert(key, Arc::new(obj));
                    Response::AllocatedObject(NewSharedObject::new(key))
                }),
            Request::DestroyObject(DestroyObject, key) => shm_objects
                .remove(&key)
                .ok_or(ResponseError::InvalidShmKey)
                .map(|_| Response::ok()),
            Request::CreateWindow(request) => {
                handle_create_window(pipe, window_ids, shm_objects, request)
            }
            Request::DestroyWindow(win_id) => 'blk: {
                if let Some(pos) = window_ids.iter().position(|id| *id == win_id) {
                    window_ids.remove(pos);
                } else {
                    break 'blk Err(ResponseError::UnknownWindow);
                }

                let mut windows = WINDOWS.lock().expect(
                    "Failed to acquire lock on Windows when cleaning up after disconnecting",
                );

                dlog!("Destroying window: {win_id}");
                if let Err(()) = windows.remove_window(win_id) {
                    wlog!("Failed to remove window {win_id}");
                }
                Ok(Response::ok())
            }
            Request::DamageWindow(damage, win_id) => {
                if let Err(()) = window::damage_window_async(
                    win_id,
                    damage.x() as usize,
                    damage.y() as usize,
                    damage.width() as usize,
                    damage.height() as usize,
                )
                .await
                {
                    Err(ResponseError::UnknownWindow)
                } else {
                    Ok(Response::ok())
                }
            }
            Request::Ping(Ping) => Ok(Response::ok()),
            Request::GetScreenInfo(GetScreenInfo) => {
                let fb_info = *FB_INFO;
                let width = fb_info.width as u32;
                let height = fb_info.height as u32;

                Ok(Response::ScreenInfo(ScreenInfo { width, height }))
            }
            Request::PreloadIcon(request) => handle_preload_icon(shm_objects, request),
            Request::LoadIcon(request, icon) => {
                if let Ok(results) = crate::icons::get_icon(icon, |data| {
                    handle_load_icon(shm_objects, request, data)
                }) {
                    results
                } else {
                    Err(ResponseError::UnknownIcon)
                }
            }
            Request::GetWindowInfo(GetWindowInfo, win_id) => {
                let windows = WINDOWS
                    .lock()
                    .expect("Failed to acquire lock on windows for a listener");
                match windows.get_window(win_id) {
                    Some(w) => {
                        let mut resp = WindowInfo::new(
                            w.x() as i32,
                            w.y() as i32,
                            w.width() as u32,
                            w.height() as u32,
                            w.flags(),
                            w.status(),
                            Name::new(w.name()).unwrap(),
                        );

                        if let Some(icon_id) = w.icon() {
                            resp = resp.with_icon_id(icon_id);
                        }

                        Ok(Response::WindowInfo(resp))
                    }
                    None => Err(ResponseError::UnknownWindow),
                }
            }
            Request::FocusWindow(FocusWindow, win_id) => {
                if !window::focus(win_id) {
                    Err(ResponseError::UnknownWindow)
                } else {
                    Ok(Response::ok())
                }
            }
        },
        Err(read_error) => match read_error {
            ReadError::DecodeError(e) => Err(e.into()),
            ReadError::Io(e) if e.kind() == ErrorKind::ConnectionAborted => {
                dlog!("One client disconnected successfully");
                return false;
            }
            ReadError::Io(e) if e.kind() == ErrorKind::WouldBlock => {
                return true;
            }
            ReadError::Io(e) => {
                elog!("Error reading from socket '{e}', disconnecting...");
                return false;
            }
        },
    };

    let response = match response {
        Err(e) => Response::err(e),
        Ok(k) => k,
    };

    write_response(&mut pipe.sender(), response).await.is_ok()
}

async fn handle_connection_async(connection: UnixSockConnection) {
    dlog!("Handling a new connection");

    let mut window_ids = Vec::with_capacity(1);
    let mut shared_objects = HashMap::with_capacity(1);

    let pipe = Arc::new(ClientComPipe::new(connection));
    // No one else is going to be receiving requests and therefore we can take ownership of the receiver
    let mut receiver = pipe.receiver();

    loop {
        if !handle_event_on(&mut window_ids, &mut shared_objects, &pipe, &mut receiver).await {
            break;
        }
    }

    // cleanup windows
    {
        let mut windows = WINDOWS
            .lock()
            .expect("Failed to acquire lock on Windows when cleaning up after disconnecting");
        for id in window_ids {
            if let Err(()) = windows.remove_window(id) {
                wlog!("Failed to remove window {id}");
            }
        }
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
    // FIXME: A very specific issue prevents from doing this correctly:
    //  env::var("OPAL_USE_THREADS")
    //       .map(|s| s.parse::<u8>().ok())
    //       .ok()
    //       .flatten()
    //       .unwrap_or(0)
    let opal_use_threads_var = 2usize;

    log!("Opal can use {} threads", opal_use_threads_var);
    let helper_threads_count =
        opal_use_threads_var.saturating_sub(2 /* this thread, and the input thread */);
    if helper_threads_count != 0 {
        for _ in 0..helper_threads_count {
            std::thread::spawn(|| executor::run(crate::window::redraw));
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
    let addr = opal_abi::CONNECT_ABSTRACT_ADDR;
    let mut listener_builder = UnixListenerBuilder::from_abstract_path(addr).unwrap();
    listener_builder
        .set_type(UnixSockKind::SeqPacket)
        .set_backlog(usize::MAX);
    listener_builder.set_non_blocking(true);

    let listener = listener_builder.bind().expect("Failed to bind a listener");
    log!("WM Listening at {}", addr);

    std::thread::spawn(|| {
        sleep(Duration::from_millis(10));
        spawn_hello();
        spawn_desktop();
        sleep(Duration::from_millis(250));
        spawn_terminal();
    });
    executor::block_on(listener_loop(listener), crate::window::redraw);
}
