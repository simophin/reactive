use std::marker::PhantomData;

use derive_builder::Builder;

use crate::{
    component::{boxed_component, BoxedComponent, Component, ComponentFactory},
    setup_context::SetupContext,
    signal::{Signal, SignalGetter},
};

pub struct Case<S> {
    create: Box<dyn for<'a> FnMut(&'a S) -> CreateState>,
}

enum CreateState {
    None,
    Created(BoxedComponent),
    MatchedUnchanged,
}

impl<S> Case<S> {
    fn new<T, C>(
        mut test: impl for<'a> FnMut(&'a S) -> Option<T> + 'static,
        mut create: impl FnMut(T) -> C + 'static,
    ) -> Self
    where
        C: Component,
        T: Clone + Eq + 'static,
    {
        let mut last_result: Option<T> = None;
        let create = move |source: &S| -> CreateState {
            let result = test(source);

            let state = match (&last_result, result) {
                (last, Some(result)) if last.as_ref() != Some(&result) => {
                    last_result.replace(result.clone());
                    CreateState::Created(boxed_component(create(result)))
                }
                (_, None) => CreateState::None,
                (_, Some(result)) => {
                    last_result.replace(result);
                    CreateState::MatchedUnchanged
                }
            };

            state
        };

        Self {
            create: Box::new(create),
        }
    }

    fn fallback(mut factory: impl ComponentFactory) -> Self {
        Self {
            create: Box::new(move |_| CreateState::Created(boxed_component(factory.create()))),
        }
    }
}

pub struct CaseBuilder<TestFn, Factory> {
    test: Option<TestFn>,
    child: Option<Factory>,
}

impl<TestFn, Factory> Default for CaseBuilder<TestFn, Factory> {
    fn default() -> Self {
        Self {
            test: None,
            child: None,
        }
    }
}

impl<TestFn, Factory> CaseBuilder<TestFn, Factory> {
    pub fn test<SourceValue, CaseInput>(self, test: TestFn) -> Self
    where
        TestFn: FnMut(&SourceValue) -> Option<CaseInput> + 'static,
    {
        Self {
            test: Some(test),
            ..self
        }
    }

    pub fn child<CaseInput, C>(self, create: Factory) -> Self
    where
        Factory: FnMut(CaseInput) -> C + 'static,
        C: Component,
    {
        Self {
            child: Some(create),
            ..self
        }
    }

    pub fn build<SourceValue, CaseInput, C>(self) -> Result<Case<SourceValue>, &'static str>
    where
        TestFn: FnMut(&SourceValue) -> Option<CaseInput> + 'static,
        Factory: FnMut(CaseInput) -> C + 'static,
        CaseInput: Clone + Eq + 'static,
        C: Component,
    {
        let test = self.test.ok_or("test is not set")?;
        let child = self.child.ok_or("child is not set")?;

        Ok(Case::new(test, child))
    }
}

pub struct FallbackBuilder<C> {
    child: Option<C>,
}

impl<T> Default for FallbackBuilder<T> {
    fn default() -> Self {
        Self { child: None }
    }
}

impl<C> FallbackBuilder<C> {
    pub fn child(self, create: C) -> Self {
        Self {
            child: Some(create),
            ..self
        }
    }

    pub fn build<S, R>(self) -> Result<Case<S>, &'static str>
    where
        C: ComponentFactory,
        R: Component,
    {
        let child = self.child.ok_or("child is not set")?;

        Ok(Case::fallback(child))
    }
}

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct Switch<S, T>
where
    T: Clone + 'static,
    S: Signal<Value = T>,
{
    source: S,
    children: Vec<Case<T>>,
}

impl<S, T> Component for Switch<S, T>
where
    T: Clone + 'static,
    S: Signal<Value = T>,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let source = self.source;
        let mut cases = self.children;

        let node_id = ctx.node_id();

        ctx.create_effect(move |ctx| {
            let source = source.get();
            let mut create_state = CreateState::None;

            for case in &mut cases {
                let state = (case.create)(&source);
                match &state {
                    CreateState::Created(_) | CreateState::MatchedUnchanged => {
                        create_state = state;
                        break;
                    }
                    CreateState::None => {}
                }
            }

            match create_state {
                CreateState::Created(child) => {
                    ctx.spawn_reactive_task(move |ctx| {
                        let node = ctx.mount_node(child);
                        if let Some(this) = ctx.find_node(node_id) {
                            this.children.clear();
                            this.children.push(node);
                        }
                    });
                }
                CreateState::MatchedUnchanged => {}
                CreateState::None => {
                    ctx.spawn_reactive_task(move |ctx| {
                        if let Some(this) = ctx.find_node(node_id) {
                            this.children.clear();
                        }
                    });
                }
            }
        });
    }
}
