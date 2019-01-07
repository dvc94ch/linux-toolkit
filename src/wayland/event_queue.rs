//! Event queue for internal use
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// An n:1 `EventQueue`.
pub struct EventQueue;

/// A cloneable `EventSource` interface to an `EventQueue`
pub struct EventSource<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
}

impl<T> Clone for EventSource<T> {
    fn clone(&self) -> EventSource<T> {
        EventSource {
            queue: self.queue.clone(),
        }
    }
}

/// An `EventDrain` interface to an `EventQueue`
pub struct EventDrain<T> {
    queue: Arc<Mutex<VecDeque<T>>>,
}

impl<T> Clone for EventDrain<T> {
    fn clone(&self) -> EventDrain<T> {
        EventDrain {
            queue: self.queue.clone(),
        }
    }
}

impl EventQueue {
    /// Returns a cloneable `EventSource` and an `EventDrain`
    pub fn new<T>() -> (EventSource<T>, EventDrain<T>) {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let source = EventSource {
            queue: queue.clone(),
        };
        let drain = EventDrain { queue: queue };
        (source, drain)
    }
}

impl<T> EventSource<T> {
    /// Pushes an event to the `EventQueue`
    pub fn push_event(&self, event: T) {
        let mut events = self.queue.lock().unwrap();
        events.push_back(event);
    }
}

impl<T> EventDrain<T> {
    /// Drains events from an `EventQueue`
    pub fn poll_events<F: FnMut(T)>(&self, mut cb: F) {
        let mut events = self.queue.lock().unwrap();
        for event in events.drain(..) {
            cb(event);
        }
    }
}
