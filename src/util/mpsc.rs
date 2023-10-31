use std::{
    cell::RefCell,
    collections::VecDeque,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Waker},
};

struct Inner<T> {
    buf: RefCell<VecDeque<T>>,
    waker: RefCell<Option<Waker>>,
}

impl<T> Inner<T> {
    fn replace_waker_if_different(&self, waker: &Waker) {
        let mut w = self.waker.borrow_mut();
        match w.as_ref() {
            Some(w) if w.will_wake(waker) => {}
            _ => *w = Some(waker.clone()),
        }
    }
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

        if let Some(waker) = inner.waker.borrow().as_ref() {
            waker.wake_by_ref();
        }

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
        let inner = self.0 .0;

        let mut buf = inner.buf.borrow_mut();
        if let Some(value) = buf.pop_front() {
            inner.replace_waker_if_different(cx.waker());
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
