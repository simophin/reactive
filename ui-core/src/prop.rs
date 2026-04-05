use reactive_core::{SetupContext, Signal};
use std::marker::PhantomData;

pub struct Prop<FrameworkType, Target, ValueType> {
    setter: fn(&Target, ValueType),
    phantom: PhantomData<fn() -> FrameworkType>,
}

impl<FrameworkType, Target, ValueType> Copy for Prop<FrameworkType, Target, ValueType> {}

impl<FrameworkType, Target, ValueType> Clone for Prop<FrameworkType, Target, ValueType> {
    fn clone(&self) -> Self {
        Self {
            setter: self.setter,
            phantom: PhantomData,
        }
    }
}

impl<FrameworkType, Target, ValueType> Prop<FrameworkType, Target, ValueType> {
    pub const fn new(setter: fn(&Target, ValueType)) -> Self {
        Self {
            setter,
            phantom: PhantomData,
        }
    }

    pub fn call(&self, target: &Target, value: ValueType) {
        (self.setter)(target, value)
    }
}

impl<FrameworkType, Target: 'static, ValueType: 'static> Prop<FrameworkType, Target, ValueType> {
    pub fn bind(
        self,
        ctx: &mut SetupContext,
        target: Target,
        signal: impl Signal<Value = ValueType> + 'static,
    ) {
        ctx.create_effect(move |_, _| {
            (self.setter)(&target, signal.read());
        })
    }
}
