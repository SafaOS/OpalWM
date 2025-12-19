use std::{
    io::{ErrorKind, Read},
    process::{Command, Stdio},
    sync::Arc,
};

use libopal::window::WindowFlags;
use opal_abi::com::{
    request::RequestKind,
    response::{
        CreateWindowResp, IconData, IconPreloaded, OkResponse, Response, ScreenInfo, WindowInfo,
        error::ResponseError,
    },
};
use safa_api::sockets::{UnixListenerBuilder, UnixSockConnection, UnixSockKind};

use crate::{
    com::{ClientComPipe, ClientComSender, ReadError},
    dlog, elog,
    framebuffer::{FB_INFO, Pixel},
    icons::icon_size,
    log, logging,
    window::{self, WINDOWS, Window, WindowKind},
    wlog,
};

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

fn write_response(sender: &mut ClientComSender, response: Response) -> Result<(), ()> {
    if let Err(e) = sender.send_response(response) {
        elog!("Error writing to socket '{e}', disconnecting...");
        return Err(());
    }
    Ok(())
}

fn handle_connect(connection: UnixSockConnection) {
    dlog!("Handling a new connection");

    let mut window_ids = Vec::with_capacity(1);

    let pipe = Arc::new(ClientComPipe::new(connection));
    // No one else is going to be receiving requests and therefore we can take ownership of the receiver
    let mut receiver = pipe.receiver();

    loop {
        let response = match receiver.receive_request() {
            Ok(req) => match req.kind() {
                RequestKind::CreateWindow(request) => match request.name() {
                    Some(name) => {
                        let height = request.height() as usize;
                        let width = request.width() as usize;
                        let flags = request.flags();

                        let abs_pos = flags.contains(WindowFlags::ABS_POS);

                        let window = Window::new_filled_with(
                            name,
                            request.icon(),
                            request.x().map(|x| x as isize).unwrap_or(0),
                            request.y().map(|y| y as isize).unwrap_or(0),
                            width,
                            height,
                            Pixel::NONE,
                            flags,
                        )
                        .with_com_pipe(pipe.clone());

                        let shm_key = *window.shm_key();

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
                                CreateWindowResp::new(id, shm_key)
                            })
                            .map(OkResponse::WindowCreated)
                            .ok_or(ResponseError::UnknownFatalError)
                    }
                    None => Err(ResponseError::InvalidUtf8),
                },
                RequestKind::DamageWindow(damage) => window::damage_window(
                    damage.win_id(),
                    damage.x() as usize,
                    damage.y() as usize,
                    damage.width() as usize,
                    damage.height() as usize,
                )
                .map(|()| OkResponse::Success)
                .map_err(|()| ResponseError::UnknownWindow),
                RequestKind::Ping => Ok(OkResponse::Success),
                RequestKind::GetScreenInfo => {
                    let fb_info = *FB_INFO;
                    let width = fb_info.width as u32;
                    let height = fb_info.height as u32;

                    Ok(OkResponse::ScreenInfo(ScreenInfo { width, height }))
                }
                RequestKind::PreloadIcon(req) => {
                    let size = req.icon_size();
                    let mut data = vec![0; size];
                    if let Err(e) = receiver.read_exact(&mut data)
                        && e.kind() != ErrorKind::ConnectionAborted
                    {
                        elog!(
                            "Failed to read {size} bytes from the client, err: {e:#?}, disconnecting..."
                        );
                        break;
                    }
                    let id = crate::icons::add_icon(data);
                    Ok(OkResponse::IconPreloaded(IconPreloaded::new(id)))
                }
                RequestKind::LoadIcon(req) => {
                    let mut sender = pipe.sender();
                    let Some(size) = icon_size(req.id()) else {
                        match write_response(&mut sender, Response::Err(ResponseError::UnknownIcon))
                        {
                            Ok(()) => continue,
                            Err(()) => break,
                        }
                    };

                    match write_response(
                        &mut sender,
                        Response::Ok(OkResponse::LoadingIcon(IconData::new(size))),
                    ) {
                        Ok(()) => (),
                        Err(()) => break,
                    }

                    if let Err(e) = crate::icons::load_icon_to(req.id(), &mut sender) {
                        elog!(
                            "Failed to write icon payload as per LoadIcon request to a client, err: {e:#?}, disconnecting..."
                        );
                        break;
                    }
                    continue;
                }
                RequestKind::GetWindowInfo(w) => {
                    let windows = WINDOWS
                        .lock()
                        .expect("Failed to acquire lock on windows for a listener");
                    match windows.get_window(w.win_id()) {
                        Some(w) => {
                            let resp = WindowInfo::new(
                                w.name(),
                                w.icon(),
                                w.x() as i32,
                                w.y() as i32,
                                w.width() as u32,
                                w.height() as u32,
                                w.flags(),
                                w.status(),
                            );

                            Ok(OkResponse::WindowInfo(resp))
                        }
                        None => Err(ResponseError::UnknownWindow),
                    }
                }
                RequestKind::FocusWindow(req) => {
                    if !window::focus(req.win_id()) {
                        Err(ResponseError::UnknownWindow)
                    } else {
                        Ok(OkResponse::Success)
                    }
                }
            },
            Err(read_error) => match read_error {
                ReadError::ParseErr(e) => Err(ResponseError::from(e)),
                ReadError::IOError(e) if e.kind() == ErrorKind::ConnectionAborted => {
                    dlog!("One client disconnected successfully");
                    break;
                }
                ReadError::IOError(e) => {
                    elog!("Error reading from socket '{e}', disconnecting...");
                    break;
                }
            },
        };

        let response = match response {
            Err(e) => Response::Err(e),
            Ok(k) => Response::Ok(k),
        };
        if let Err(()) = write_response(&mut pipe.sender(), response) {
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

/// Listens for incoming connections and handles them
pub fn listen() -> ! {
    let addr = opal_abi::CONNECT_ABSTRACT_ADDR;
    let mut listener_builder = UnixListenerBuilder::from_abstract_path(addr).unwrap();
    listener_builder
        .set_type(UnixSockKind::SeqPacket)
        .set_backlog(usize::MAX);

    let listener = listener_builder.bind().expect("Failed to bind a listener");
    log!("WM Listening at {}", addr);

    spawn_hello();
    spawn_desktop();

    loop {
        let connection = listener
            .accept()
            .expect("Failed to Accept a pending connection");

        // TODO: Implement something similar to poll
        std::thread::spawn(|| handle_connect(connection));
    }
}
