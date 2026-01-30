//! The Asynchronous Executor for executing tasks that depend on resources and polling events efficiently.

use std::{
    cell::UnsafeCell,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, Sender, TryRecvError},
    },
    task::{Context, Waker},
};

use crate::vtty::{self, ChildVTTY, MotherVTTY};

use futures_util::{
    FutureExt,
    future::BoxFuture,
    task::{ArcWake, waker_ref},
};
use safa_api::abi::poll::PollEntry;

use crate::window::redraw;

#[derive(Debug)]
struct IOPoller {
    current_events: Vec<PollEntry>,
    current_wakers: Vec<Waker>,
}

impl IOPoller {
    pub const fn new() -> Self {
        Self {
            current_events: Vec::new(),
            current_wakers: Vec::new(),
        }
    }

    pub fn poll(&mut self) {
        assert_eq!(
            self.current_events.len(),
            self.current_wakers.len(),
            "Mismatched number of events and wakers"
        );

        if self.current_events.is_empty() {
            crate::thread::yield_now();
            return;
        }

        safa_api::syscalls::io::poll_resources(self.current_events.as_mut_slice(), None)
            .expect("Poll Failed for some reason");

        let mut i = 0;
        while i < self.current_events.len() {
            let event = &mut self.current_events[i];

            if !event.returned_events().is_empty() {
                let waker = self.current_wakers.swap_remove(i);
                waker.wake();

                self.current_events.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    pub fn add_poll(&mut self, poll_entry: PollEntry, waker: Waker) {
        self.current_events.push(poll_entry);
        self.current_wakers.push(waker);
    }
}

#[derive(Debug)]
struct Task {
    task_sender: Sender<Arc<Self>>,
    /// Safety: Executor runs from a single thread at a time
    future: UnsafeCell<BoxFuture<'static, ()>>,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self
            .task_sender
            .send(arc_self.clone())
            .expect("Failed to send task")
    }
}

struct Spawner {
    task_sender: Vec<(Sender<Arc<Task>>, ChildVTTY)>,
    next_sender: usize,
}

impl Spawner {
    pub const fn new() -> Self {
        Self {
            task_sender: Vec::new(),
            next_sender: 0,
        }
    }

    /// Adds a new sender to the spawner
    pub fn add_sender(&mut self, sender: Sender<Arc<Task>>, vtty: ChildVTTY) -> usize {
        self.task_sender.push((sender, vtty));
        self.task_sender.len() - 1
    }

    #[inline(always)]
    fn get_next_sender(&mut self) -> usize {
        let sender_index = self.next_sender % self.task_sender.len();
        self.next_sender = self.next_sender.wrapping_add(1);

        sender_index
    }

    pub fn spawn(
        &mut self,
        future: impl Future<Output = ()> + 'static + Send,
        spec_sender: Option<usize>,
    ) {
        let sender_index = spec_sender.unwrap_or_else(|| self.get_next_sender());
        let (sender, vtty) = &self.task_sender[sender_index];
        let task = Arc::new(Task {
            task_sender: sender.clone(),
            future: UnsafeCell::new(future.boxed()),
        });

        sender.send(task).expect("Failed to send task");
        vtty.write(0, b"wakeup")
            .expect("Failed to send wakeup signal");
    }
}

struct Executor {
    io_poller: UnsafeCell<IOPoller>,
    task_queue: Receiver<Arc<Task>>,
    executor_id: usize,
    running: AtomicBool,
}

impl Executor {
    #[inline]
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        SPAWNER
            .lock()
            .expect("Failed to acquire lock on spawner")
            .spawn(future, Some(self.executor_id));
    }

    /// Runs the executor until all the tasks are completed.
    pub fn run(&self) {
        if self
            .running
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("Attempt to block_on async context")
        }

        loop {
            let task_result = self.task_queue.try_recv();
            match task_result {
                Ok(task) => {
                    let waker = waker_ref(&task);
                    let mut context = Context::from_waker(&*waker);

                    let future = unsafe { &mut *task.future.get() };
                    match future.as_mut().poll(&mut context) {
                        std::task::Poll::Pending => {}
                        std::task::Poll::Ready(()) => {}
                    }
                }
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) if !redraw() => {
                    // IO Poll until we get task
                    unsafe { &mut *self.io_poller.get() }.poll()
                }
                Err(TryRecvError::Empty) => {}
            }
        }
        self.running.store(false, Ordering::Relaxed);
    }
}

static SPAWNER: Mutex<Spawner> = Mutex::new(Spawner::new());

thread_local! {
    pub static ASYNC_CONTEXT: Executor = {
        let (sender, receiver) = std::sync::mpsc::channel();
        let (vtty_m, spawner_vtty) = vtty::new();

        vtty_m.set_flags(0).expect("Failed to set VTTY mode to silent");
        let executor_id = SPAWNER.lock().expect("Failed to acquire lock on spawner").add_sender(sender, spawner_vtty);


        let executor = Executor {
            io_poller: UnsafeCell::new(IOPoller::new()),
            task_queue: receiver,
            executor_id,
            running: AtomicBool::new(false),
        };

        let handle_messages = handle_messages(vtty_m);
        executor.spawn(handle_messages);

        executor
    };
}

/// Spawns a new async task
#[inline]
pub fn spawn(future: impl Future<Output = ()> + 'static + Send) {
    SPAWNER
        .lock()
        .expect("Failed to acquire lock on spawner")
        .spawn(future, None)
}

async fn handle_messages(vtty: MotherVTTY) {
    let mut buf = [0u8; 64];
    loop {
        vtty.read_async(&mut buf)
            .await
            .expect("Failed to read messages for an executor");
        // TODO: do more message handling here and read buf
        _ = buf;
    }
}

/// Spawns an executor to execute async tasks, with the given main task.
pub fn block_on(future: impl Future<Output = ()> + Send + 'static) {
    ASYNC_CONTEXT.with(|executor| {
        executor.spawn(future);
        executor.run();
    });
}

/// Runs the async executor with no main Task.
///
/// Adds the thread to the executors multi-threaded task hivemind.
pub fn run() {
    ASYNC_CONTEXT.with(|executor| {
        executor.run();
    });
}

/// Poll a resource in a currently async context, sleeps until resource is ready.
pub fn poll_resource(poll_entry: PollEntry, waker: Waker) {
    ASYNC_CONTEXT.with(|executor| {
        unsafe { &mut *executor.io_poller.get() }.add_poll(poll_entry, waker);
    });
}
