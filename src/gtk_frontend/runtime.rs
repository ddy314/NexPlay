use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use crate::app::AppContext;
use crate::backend_api::{BackendEvent, frontend_event_from_app};
use crate::error::AppResult;
use crate::task::AppEvent;

const WORKER_COUNT: usize = 4;
const JOB_QUEUE_CAPACITY: usize = 32;
const RESULT_QUEUE_CAPACITY: usize = 64;

type Job = Box<dyn FnOnce(&AppContext) -> WorkerCompletion + Send + 'static>;
type CompletionCallback = Box<dyn FnOnce(Result<Box<dyn Any + Send>, String>) + 'static>;

struct WorkerCompletion {
    id: u64,
    result: Result<Box<dyn Any + Send>, String>,
}

/// GTK owns this type on the main thread.  Backend jobs are sent through a
/// small bounded queue and only owned Rust values cross back into the UI.
pub struct BackendRuntime {
    jobs: mpsc::SyncSender<Job>,
    results: Arc<Mutex<mpsc::Receiver<WorkerCompletion>>>,
    events: Arc<Mutex<mpsc::Receiver<BackendEvent>>>,
    callbacks: RefCell<HashMap<u64, CompletionCallback>>,
    next_id: Cell<u64>,
}

impl BackendRuntime {
    pub fn new(context: AppContext) -> Self {
        let (jobs, job_receiver) = mpsc::sync_channel::<Job>(JOB_QUEUE_CAPACITY);
        let job_receiver = Arc::new(Mutex::new(job_receiver));
        let (result_sender, result_receiver) =
            mpsc::sync_channel::<WorkerCompletion>(RESULT_QUEUE_CAPACITY);
        let results = Arc::new(Mutex::new(result_receiver));

        for index in 0..WORKER_COUNT {
            let context = context.clone();
            let job_receiver = job_receiver.clone();
            let result_sender = result_sender.clone();
            let _ = thread::Builder::new()
                .name(format!("nexplay-gtk-worker-{index}"))
                .spawn(move || {
                    loop {
                        let job = {
                            let receiver = job_receiver
                                .lock()
                                .expect("GTK backend job receiver mutex poisoned");
                            receiver.recv()
                        };
                        let Ok(job) = job else { break };
                        let completion = job(&context);
                        if result_sender.send(completion).is_err() {
                            break;
                        }
                    }
                });
        }

        let (event_sender, event_receiver) = mpsc::sync_channel(RESULT_QUEUE_CAPACITY);
        if let Some(receiver) = context
            .event_receiver
            .lock()
            .expect("GTK event receiver mutex poisoned")
            .take()
        {
            let _ = thread::Builder::new()
                .name("nexplay-gtk-event-pump".to_string())
                .spawn(move || {
                    for event in receiver {
                        let event = frontend_event_from_app(event);
                        if event_sender.send(event).is_err() {
                            break;
                        }
                    }
                });
        }

        Self {
            jobs,
            results,
            events: Arc::new(Mutex::new(event_receiver)),
            callbacks: RefCell::new(HashMap::new()),
            next_id: Cell::new(1),
        }
    }

    pub fn submit<T, F, C>(&self, work: F, callback: C)
    where
        T: Send + 'static,
        F: FnOnce(&AppContext) -> AppResult<T> + Send + 'static,
        C: FnOnce(Result<T, String>) + 'static,
    {
        let id = self.next_id.get();
        self.next_id.set(id.saturating_add(1));
        self.callbacks.borrow_mut().insert(
            id,
            Box::new(move |result| {
                let result = result.and_then(|value| {
                    value
                        .downcast::<T>()
                        .map(|value| *value)
                        .map_err(|_| "GTK backend returned an unexpected result".to_string())
                });
                callback(result);
            }),
        );

        let job = Box::new(move |context: &AppContext| WorkerCompletion {
            id,
            result: work(context)
                .map(|value| Box::new(value) as Box<dyn Any + Send>)
                .map_err(|error| error.to_string()),
        });
        if let Err(error) = self.jobs.try_send(job) {
            let message = match error {
                mpsc::TrySendError::Full(_) => {
                    "GTK backend worker queue is full; try again shortly".to_string()
                }
                mpsc::TrySendError::Disconnected(_) => {
                    "GTK backend worker pool stopped".to_string()
                }
            };
            let callback = { self.callbacks.borrow_mut().remove(&id) };
            if let Some(callback) = callback {
                callback(Err(message));
            }
        }
    }

    /// Dispatch completed jobs.  Call this from a GLib timeout on the GTK
    /// main context; no GTK object is ever touched by a worker.
    pub fn poll(&self) {
        let mut completions = Vec::new();
        let receiver = self
            .results
            .lock()
            .expect("GTK backend result receiver mutex poisoned");
        loop {
            match receiver.try_recv() {
                Ok(completion) => completions.push(completion),
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
            }
        }
        drop(receiver);

        for completion in completions {
            let callback = { self.callbacks.borrow_mut().remove(&completion.id) };
            if let Some(callback) = callback {
                callback(completion.result);
            }
        }
    }

    pub fn drain_events(&self) -> Vec<BackendEvent> {
        let receiver = self
            .events
            .lock()
            .expect("GTK event receiver mutex poisoned");
        let mut events = Vec::new();
        loop {
            match receiver.try_recv() {
                Ok(event) => events.push(event),
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
            }
        }
        events
    }

    pub fn context_is_alive(&self) -> bool {
        // Kept as a named diagnostic hook for the UI and future graceful
        // shutdown work.  The worker queue owns the cloned AppContext.
        !self.callbacks.borrow().is_empty()
    }

    #[allow(dead_code)]
    fn _event_type_is_send(_: AppEvent) {}
}
