use std::{
    cell::RefCell,
    collections::VecDeque,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll},
};

use local_waker::LocalWaker;

struct Inner<T> {
    buf: RefCell<VecDeque<T>>,
    waker: LocalWaker,
}

pub struct Sender<T>(Weak<Inner<T>>);

pub enum SendError<T> {
    Full(T),
    Closed(T),
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        let Some(inner) = self.0.upgrade() else {
            return Err(SendError::Closed(value));
        };

        let mut buf = inner.buf.borrow_mut();

        if buf.capacity() == buf.len() {
            return Err(SendError::Full(value));
        }

        buf.push_back(value);

        inner.waker.wake();
        Ok(())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender(self.0.clone())
    }
}

pub struct Receiver<T>(Rc<Inner<T>>);

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> impl Future<Output = Option<T>> + '_ {
        ReceiverNext(self)
    }
}

struct ReceiverNext<'a, T>(&'a mut Receiver<T>);

impl<'a, T> Future for ReceiverNext<'a, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.0 .0.as_ref();

        inner.waker.register(cx.waker());

        if let Some(value) = inner.buf.borrow_mut().pop_front() {
            cx.waker().wake_by_ref();
            return Poll::Ready(Some(value));
        }

        Poll::Pending
    }
}

pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Rc::new(Inner {
        buf: RefCell::new(VecDeque::with_capacity(cap)),
        waker: Default::default(),
    });

    let sender = Sender(Rc::downgrade(&inner));
    let receiver = Receiver(inner);

    (sender, receiver)
}
