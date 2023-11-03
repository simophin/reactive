use std::{cell::RefCell, collections::HashMap, rc::Rc};

use futures::Future;
use local_waker::LocalWaker;

use crate::react_context::SignalID;

use super::signal_set::SignalSet;

type SubscriberID = usize;

#[derive(Default)]
struct State {
    subscribers: HashMap<SubscriberID, Subscriber>,
}

struct Subscriber {
    waker: LocalWaker,
    subscribing: SignalSet,
    pending_received: bool,
}

#[derive(Clone, Default)]
pub struct Sender {
    state: Rc<RefCell<State>>,
}

impl Sender {
    pub fn send(&self, signal: SignalID) {
        let mut state = self.state.borrow_mut();
        for subscriber in state.subscribers.values_mut() {
            if subscriber.subscribing.contains(signal) {
                subscriber.pending_received = true;
                subscriber.waker.wake();
            }
        }
    }

    pub fn subscribe(&self) -> Receiver {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let mut state = self.state.borrow_mut();
        if state
            .subscribers
            .insert(
                id,
                Subscriber {
                    waker: LocalWaker::new(),
                    subscribing: SignalSet::default(),
                    pending_received: false,
                },
            )
            .is_some()
        {
            panic!("Subscriber ID collision");
        }

        Receiver {
            state: self.state.clone(),
            id,
        }
    }
}

pub struct Receiver {
    state: Rc<RefCell<State>>,
    id: SubscriberID,
}

impl Receiver {
    pub fn set_subscribing(&mut self, signals: impl Iterator<Item = SignalID>) {
        let mut state = self.state.borrow_mut();
        if let Some(subscriber) = state.subscribers.get_mut(&self.id) {
            for signal in signals.into_iter() {
                subscriber.subscribing.insert(signal);
            }
            subscriber.waker.wake();
        }
    }

    pub fn next(&mut self) -> impl Future<Output = Option<()>> + Unpin + '_ {
        ReceiverPoll(self)
    }

    pub fn sender(&self) -> Sender {
        Sender {
            state: self.state.clone(),
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        let mut state = self.state.borrow_mut();
        if state.subscribers.remove(&self.id).is_none() {
            log::error!("Subscriber is missing when dropping receiver");
        }
    }
}

struct ReceiverPoll<'a>(&'a mut Receiver);

impl<'a> Future for ReceiverPoll<'a> {
    type Output = Option<()>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut state = self.0.state.borrow_mut();
        if let Some(subscriber) = state.subscribers.get_mut(&self.0.id) {
            subscriber.waker.register(cx.waker());
            if subscriber.pending_received {
                subscriber.pending_received = false;
                std::task::Poll::Ready(Some(()))
            } else {
                std::task::Poll::Pending
            }
        } else {
            std::task::Poll::Ready(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };

    use super::*;

    #[test]
    fn channel_works() {
        let sender = Sender::default();

        let mut receiver1 = sender.subscribe();
        receiver1.set_subscribing([1, 2].into_iter());

        let mut receiver2 = sender.subscribe();
        receiver2.set_subscribing([3].into_iter());

        sender.send(1);
        assert_eq!(next_value(&mut receiver1.next()), Poll::Ready(Some(())));
        assert_eq!(next_value(&mut receiver2.next()), Poll::Pending);

        sender.send(3);
        assert_eq!(next_value(&mut receiver1.next()), Poll::Pending);
        assert_eq!(next_value(&mut receiver2.next()), Poll::Ready(Some(())));
    }

    fn next_value(future: &mut (impl Future<Output = Option<()>> + Unpin)) -> Poll<Option<()>> {
        Pin::new(future).poll(&mut Context::from_waker(futures::task::noop_waker_ref()))
    }
}
