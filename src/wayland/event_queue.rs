use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct EventQueue;

#[derive(Clone)]
pub struct EventSource<T> {
    queue: Arc<Mutex<VecDeque<T>>>
}

#[derive(Clone)]
pub struct EventDrain<T> {
    queue: Arc<Mutex<VecDeque<T>>>
}

impl EventQueue {
    pub fn new<T>() -> (EventSource<T>, EventDrain<T>) {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let source = EventSource {
            queue: queue.clone()
        };
        let drain = EventDrain {
            queue: queue
        };
        (source, drain)
    }
}

impl<T> EventSource<T> {
    pub fn push_event(&self, event: T) {
        let mut events = self.queue.lock().unwrap();
        events.push_back(event);
    }
}

impl<T> EventDrain<T> {
    pub fn poll_events<F: FnMut(T)>(&self, mut cb: F) {
        let mut events = self.queue.lock().unwrap();
        for event in events.drain(..) {
            cb(event);
        }
    }
}
