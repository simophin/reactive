use std::{
    cell::RefCell,
    collections::{BTreeSet, HashMap},
    pin::{pin, Pin},
    rc::Rc,
    task::{Context, Poll},
};

use async_broadcast::Sender;
use futures::Future;
use linked_hash_set::LinkedHashSet;

use crate::{
    effect::{BoxedEffect, EffectCleanup},
    effect_context::EffectContext,
    task::WeakTask,
    tracker::Tracker,
    util::{
        diff::{diff_sorted, DiffResult},
        mpsc,
    },
};

pub type SignalID = usize;
pub type EffectID = usize;

pub struct ReactiveContext<F> {
    effect_id_seq: EffectID,
    effects: HashMap<EffectID, EffectStateRef>,
    signal_deps: HashMap<SignalID, LinkedHashSet<EffectID>>,

    task_sender: mpsc::Sender<WeakTask>,

    running_future: F,
}

impl<F> ReactiveContext<F> {
    pub fn new() -> ReactiveContext<impl Future + 'static> {
        let (task_sender, mut task_rx) = mpsc::channel(512);

        ReactiveContext {
            effect_id_seq: 0,
            effects: Default::default(),
            signal_deps: Default::default(),
            task_sender,
            running_future: async move {
                while let Some(task) = task_rx.next().await {
                    task.await;
                }
            },
        }
    }
}

impl<F> ReactiveContext<F> {
    pub fn new_effect(&mut self, effect: BoxedEffect) -> EffectID {
        let id = self.effect_id_seq;
        self.effect_id_seq += 1;
        self.effects.insert(
            id,
            Rc::new(RefCell::new(EffectState {
                id,
                effect,
                last_clean_up: None,
                last_tracked_signals: Default::default(),
            })),
        );
        id
    }

    pub fn poll(&mut self) -> ReactiveContextPoll<'_, F> {
        ReactiveContextPoll { ctx: self }
    }
}

pub struct ReactiveContextPoll<'a, F> {
    ctx: &'a mut ReactiveContext<F>,
}

impl<'a, F: Future> Future for ReactiveContextPoll<'a, F>
where
    F: Future,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut future = &mut self.ctx.running_future;
        pin!(future);
        // Pin::new(future).poll(cx)
        todo!()
    }
}

pub struct EffectState {
    pub id: EffectID,
    pub effect: BoxedEffect,
    pub last_clean_up: Option<Box<dyn EffectCleanup>>,
    pub last_tracked_signals: BTreeSet<SignalID>,
}

pub type EffectStateRef = Rc<RefCell<EffectState>>;

impl EffectState {
    pub fn run(&mut self) -> DiffResult<SignalID> {
        if let Some(mut clean_up) = self.last_clean_up.take() {
            clean_up.cleanup();
        }

        let mut ctx = EffectContext::new();
        Tracker::set_current(Some(Default::default()));
        self.last_clean_up.replace(self.effect.run(&mut ctx));
        let tracker = Tracker::set_current(None).expect("To have tracker set before");

        let result = diff_sorted(
            self.last_tracked_signals.iter().cloned(),
            tracker.iter().cloned(),
        );

        self.last_tracked_signals = tracker.into_inner();
        result
    }
}

impl Drop for EffectState {
    fn drop(&mut self) {
        if let Some(mut clean_up) = self.last_clean_up.take() {
            clean_up.cleanup();
        }
    }
}

#[derive(Clone)]
pub struct SignalNotifier(SignalID, Sender<SignalID>);

impl SignalNotifier {
    pub fn new(id: SignalID, sender: Sender<SignalID>) -> Self {
        Self(id, sender)
    }

    pub fn notify_changed(&mut self) {
        match self.1.try_broadcast(self.0) {
            Ok(_) => {}
            Err(e) => {
                log::warn!("Unable to delivery signal changes due to: {e:?}");
            }
        }
    }
}
