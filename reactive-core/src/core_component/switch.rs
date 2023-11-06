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

    fn fallback(mut factory: ComponentFactory) -> Self {
        Self {
            create: Box::new(move |_| CreateState::Created(factory.create())),
        }
    }
}

pub struct CaseBuilder<TestFn, Factory> {
    test: Option<TestFn>,
    children: Option<Factory>,
}

impl<TestFn, Factory> Default for CaseBuilder<TestFn, Factory> {
    fn default() -> Self {
        Self {
            test: None,
            children: None,
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
            children: Some(create),
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
        let child = self.children.ok_or("child is not set")?;

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

        ctx.create_effect(move |ctx| {
            let source = source.get();
            let mut create_state = CreateState::None;

            for case in cases.iter_mut().chain(std::iter::once(&mut fallback.0)) {
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
