use objc2::rc::Retained;
use reactive_core::{SetupContext, Signal};
use std::marker::PhantomData;

pub struct Prop<RustType, ObjCType, ObjCValueType> {
    setter: fn(&Retained<ObjCType>, ObjCValueType),
    phantom: PhantomData<fn() -> RustType>,
}

impl<RustType, ObjCType, ObjCValueType> Copy for Prop<RustType, ObjCType, ObjCValueType> {}

impl<RustType, ObjCType, ObjCValueType> Clone for Prop<RustType, ObjCType, ObjCValueType> {
    fn clone(&self) -> Self {
        Self {
            setter: self.setter,
            phantom: PhantomData,
        }
    }
}

impl<RustType, ObjCType, ObjCValueType> Prop<RustType, ObjCType, ObjCValueType> {
    pub const fn new(setter: fn(&Retained<ObjCType>, ObjCValueType)) -> Self {
        Self {
            setter,
            phantom: PhantomData,
        }
    }
}

impl<RustType, ObjCType: 'static, ObjcValueType: 'static> Prop<RustType, ObjCType, ObjcValueType> {
    pub fn bind(
        self,
        ctx: &mut SetupContext,
        v: Retained<ObjCType>,
        s: impl Signal<Value = ObjcValueType> + 'static,
    ) {
        ctx.create_effect(move |_, _| {
            (self.setter)(&v, s.read());
        })
    }
}
