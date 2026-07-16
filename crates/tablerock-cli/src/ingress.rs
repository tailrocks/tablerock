//! Bounded post-mapping delivery with loss policy explicit in its types.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};

use tokio::sync::Notify;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery<E, P> {
    Event(E),
    Progress(P),
    ResyncRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome {
    Accepted,
    ResyncRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryReceiveError {
    Empty,
    Closed,
}

pub struct IngressSender<E, P> {
    shared: Arc<Shared<E, P>>,
}

pub struct IngressReceiver<E, P> {
    shared: Arc<Shared<E, P>>,
}

struct Shared<E, P> {
    state: Mutex<State<E, P>>,
    ready: Notify,
}

struct State<E, P> {
    events: VecDeque<E>,
    progress: Option<P>,
    resync_required: bool,
    sender_count: usize,
    receiver_open: bool,
    capacity: usize,
}

#[must_use]
pub fn bounded_ingress<E, P>(capacity: usize) -> (IngressSender<E, P>, IngressReceiver<E, P>) {
    assert!(capacity > 0, "ingress capacity must be positive");
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            events: VecDeque::with_capacity(capacity),
            progress: None,
            resync_required: false,
            sender_count: 1,
            receiver_open: true,
            capacity,
        }),
        ready: Notify::new(),
    });
    (
        IngressSender {
            shared: Arc::clone(&shared),
        },
        IngressReceiver { shared },
    )
}

impl<E, P> IngressSender<E, P> {
    pub fn try_send_event(&self, event: E) -> Result<SendOutcome, E> {
        let mut state = lock_state(&self.shared.state);
        if !state.receiver_open {
            return Err(event);
        }
        let outcome = if state.events.len() == state.capacity {
            state.resync_required = true;
            SendOutcome::ResyncRequired
        } else {
            state.events.push_back(event);
            SendOutcome::Accepted
        };
        drop(state);
        self.shared.ready.notify_one();
        Ok(outcome)
    }

    pub fn publish_progress(&self, progress: P) -> Result<(), P> {
        let mut state = lock_state(&self.shared.state);
        if !state.receiver_open {
            return Err(progress);
        }
        state.progress = Some(progress);
        drop(state);
        self.shared.ready.notify_one();
        Ok(())
    }
}

impl<E, P> Clone for IngressSender<E, P> {
    fn clone(&self) -> Self {
        lock_state(&self.shared.state).sender_count += 1;
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<E, P> Drop for IngressSender<E, P> {
    fn drop(&mut self) {
        let mut state = lock_state(&self.shared.state);
        state.sender_count -= 1;
        let closed = state.sender_count == 0;
        drop(state);
        if closed {
            self.shared.ready.notify_one();
        }
    }
}

impl<E, P> IngressReceiver<E, P> {
    pub fn try_recv(&mut self) -> Result<Delivery<E, P>, TryReceiveError> {
        let mut state = lock_state(&self.shared.state);
        if state.resync_required {
            state.resync_required = false;
            return Ok(Delivery::ResyncRequired);
        }
        if let Some(event) = state.events.pop_front() {
            return Ok(Delivery::Event(event));
        }
        if let Some(progress) = state.progress.take() {
            return Ok(Delivery::Progress(progress));
        }
        if state.sender_count == 0 {
            Err(TryReceiveError::Closed)
        } else {
            Err(TryReceiveError::Empty)
        }
    }

    pub async fn recv(&mut self) -> Option<Delivery<E, P>> {
        loop {
            let shared = Arc::clone(&self.shared);
            let ready = shared.ready.notified();
            match self.try_recv() {
                Ok(delivery) => return Some(delivery),
                Err(TryReceiveError::Closed) => return None,
                Err(TryReceiveError::Empty) => ready.await,
            }
        }
    }
}

impl<E, P> Drop for IngressReceiver<E, P> {
    fn drop(&mut self) {
        let mut state = lock_state(&self.shared.state);
        state.receiver_open = false;
        state.events.clear();
        state.progress = None;
        state.resync_required = false;
    }
}

fn lock_state<E, P>(mutex: &Mutex<State<E, P>>) -> MutexGuard<'_, State<E, P>> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
