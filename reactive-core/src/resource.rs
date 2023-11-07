use futures::Future;

use crate::{
    signal::{Signal, SignalGetter},
    Effect, EffectContext, SignalWriter,
};

#[derive(Debug, Clone)]
pub struct Resource<T> {
    pub value: Option<T>,
    pub state: LoadState,
}

impl<T> Default for Resource<T> {
    fn default() -> Self {
        Self {
            value: None,
            state: LoadState::Idle,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadState {
    Idle,
    Loading,
    Loaded,
}

pub(crate) fn new_resource_effect<SI, T, FutT, F>(
    signal_input: SI,
    out_w: SignalWriter<Resource<T>>,
    mut factory: F,
) -> impl Effect
where
    SI: Signal,
    <SI as Signal>::Value: Clone,
    T: 'static,
    FutT: Future<Output = T> + 'static,
    F: ResourceFactory<<SI as Signal>::Value, FutT>,
{
    move |ctx: &mut EffectContext| {
        let input = signal_input.get();
        let mut out_w = out_w.clone();
        out_w.update_with(|state| {
            state.state = LoadState::Loading;
            true
        });

        let fut = factory.create(input);
        ctx.spawn(async move {
            let value = fut.await;
            out_w.update_with(|state| {
                state.value.replace(value);
                state.state = LoadState::Loaded;
                true
            });
        });
    }
}

pub trait ResourceFactory<Input: 'static, Output: 'static>: 'static {
    fn create(&mut self, input: Input) -> Output;
}

impl<F, I, O> ResourceFactory<I, O> for F
where
    F: FnMut(I) -> O + 'static,
    I: 'static,
    O: 'static,
{
    fn create(&mut self, input: I) -> O {
        (self)(input)
    }
}
