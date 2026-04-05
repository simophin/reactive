use crate::Prop;
use reactive_core::{SetupContext, Signal};
use std::rc::Rc;

pub struct ViewBuilder<Target> {
    create_target: Box<dyn FnOnce(&mut SetupContext) -> Target>,
    property_binders: Vec<Box<dyn FnOnce(&mut SetupContext, Target)>>,
    /// Called after every property setter fires (e.g. to invalidate parent layout).
    after_set: Rc<dyn Fn(&Target) + 'static>,
}

impl<Target> ViewBuilder<Target>
where
    Target: Clone + 'static,
{
    pub fn new(creator: impl FnOnce(&mut SetupContext) -> Target + 'static) -> Self {
        Self {
            create_target: Box::new(creator),
            property_binders: vec![],
            after_set: Rc::new(|_| {}),
        }
    }

    /// Set a callback that runs after every prop setter fires.
    ///
    /// Must be called before any [`bind`][Self::bind] calls so that the
    /// captured `Rc` inside each effect points to this closure.
    pub fn set_after_set(&mut self, f: impl Fn(&Target) + 'static) {
        self.after_set = Rc::new(f);
    }

    pub fn bind<FrameworkType, ValueType>(
        &mut self,
        prop: &'static Prop<FrameworkType, Target, ValueType>,
        signal: impl Signal<Value = ValueType> + 'static,
    ) {
        let after = Rc::clone(&self.after_set);
        self.property_binders.push(Box::new(move |ctx, target| {
            ctx.create_effect(move |_, _| {
                prop.call(&target, signal.read());
                after(&target);
            });
        }));
    }

    pub fn setup(self, ctx: &mut SetupContext) -> Target {
        let target = (self.create_target)(ctx);
        for binder in self.property_binders {
            binder(ctx, target.clone());
        }
        target
    }
}
