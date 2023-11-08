use derive_builder::Builder;

use crate::{
    component::{boxed_component, BoxedComponent, Component, ComponentFactory},
    setup_context::SetupContext,
    signal::{Signal, SignalGetter},
};

use super::testable::Testable;

pub struct Case<S> {
    create: Box<dyn for<'a> FnMut(&'a S, bool) -> CreateState>,
}

enum CreateState {
    None,
    Created(BoxedComponent),
    MatchedUnchanged,
}

impl<S> Case<S> {
    fn new<T, C>(
        mut test: impl for<'a> FnMut(&'a S) -> T + 'static,
        mut create: impl FnMut(T::Value) -> C + 'static,
    ) -> Self
    where
        C: Component,
        T: Testable + Clone + Eq,
    {
        let mut last_result: Option<T> = None;
        let create = move |source: &S, force: bool| -> CreateState {
            let result = test(source);

            let state = match (&last_result, result, force) {
                (_, r, _) if !r.is_ok() => CreateState::None,
                (Some(last), r, false) if last == &r => CreateState::MatchedUnchanged,

                (_, r, _) => {
                    last_result.replace(r.clone());
                    CreateState::Created(boxed_component(create(r.to_output())))
                }
            };

            state
        };

        Self {
            create: Box::new(create),
        }
    }

    fn fallback(mut factory: ComponentFactory) -> Self {
        Self {
            create: Box::new(move |_, force: bool| {
                if force {
                    CreateState::Created(factory.create())
                } else {
                    CreateState::MatchedUnchanged
                }
            }),
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
        TestFn: FnMut(&SourceValue) -> CaseInput + 'static,
        CaseInput: Testable + Clone + Eq,
    {
        Self {
            test: Some(test),
            ..self
        }
    }

    pub fn child<Arg, C>(self, create: Factory) -> Self
    where
        Factory: FnMut(Arg) -> C + 'static,
        C: Component,
    {
        Self {
            child: Some(create),
            ..self
        }
    }

    pub fn build<SourceValue, CaseInput, C>(self) -> Result<Case<SourceValue>, &'static str>
    where
        TestFn: FnMut(&SourceValue) -> CaseInput + 'static,
        Factory: FnMut(CaseInput::Value) -> C + 'static,
        CaseInput: Testable + Clone + Eq,
        C: Component,
    {
        let test = self.test.ok_or("test is not set")?;
        let child = self.child.ok_or("child is not set")?;

        Ok(Case::new(test, child))
    }
}

pub struct Fallback<S>(Case<S>);

impl<S, F: FnMut() -> C + 'static, C: Component> From<F> for Fallback<S> {
    fn from(factory: F) -> Self {
        Self(Case::fallback(factory.into()))
    }
}

impl<S> Default for Fallback<S> {
    fn default() -> Self {
        Self(Case::fallback(ComponentFactory::empty()))
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
    #[builder(setter(into), default)]
    fallback: Fallback<T>,
    children: Vec<Case<T>>,
}

impl<S, T> SwitchBuilder<S, T>
where
    T: Clone + 'static,
    S: Signal<Value = T>,
{
    pub fn child(self, child: Case<T>) -> Self {
        let mut children = self.children.unwrap_or_default();
        children.clear();
        children.push(child);
        Self {
            children: Some(children),
            ..self
        }
    }
}

impl<S, T> Component for Switch<S, T>
where
    T: Clone + 'static,
    S: Signal<Value = T>,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let source = self.source;
        let mut cases = self.children;
        let mut fallback = self.fallback;
        let mut last_match_index: Option<usize> = None;

        ctx.create_effect_fn(move |ctx| {
            let source = source.get();
            let mut create_state = CreateState::None;

            for (index, case) in cases
                .iter_mut()
                .chain(std::iter::once(&mut fallback.0))
                .enumerate()
            {
                let state =
                    (case.create)(&source, last_match_index.is_some_and(|last| last != index));

                match &state {
                    CreateState::Created(_) | CreateState::MatchedUnchanged => {
                        create_state = state;
                        last_match_index.replace(index);
                        break;
                    }
                    CreateState::None => {}
                }
            }

            match create_state {
                CreateState::Created(child) => {
                    let child = ctx.mount_node(child);
                    ctx.with_current_node(move |node| {
                        node.children.clear();
                        node.children.push(child);
                    });
                }
                CreateState::MatchedUnchanged => {}
                CreateState::None => {
                    ctx.with_current_node(move |node| {
                        node.children.clear();
                    });
                }
            }
        });
    }
}
