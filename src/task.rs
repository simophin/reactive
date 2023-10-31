use std::{
    cell::RefCell,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll},
};

use futures::Future;

pub struct WeakTask(Weak<RefCell<Box<dyn Future<Output = ()> + Unpin>>>);

impl WeakTask {
    pub fn new(
        future: impl Future<Output = ()> + Unpin + 'static,
    ) -> Rc<RefCell<Box<dyn Future<Output = ()>>>> {
        todo!()
    }
}

impl Future for WeakTask {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0.upgrade() {
            Some(future) => {
                let mut borrow = future.borrow_mut();
                let future = &mut *borrow;
                let future = future.as_mut();
                Pin::new(future).poll(cx)
            }

            None => Poll::Ready(()),
        }
    }
}
