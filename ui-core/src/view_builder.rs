use crate::Prop;
use reactive_core::{SetupContext, Signal};

pub struct ViewBuilder<Target> {
    create_target: Box<dyn FnOnce(&mut SetupContext) -> Target>,
    property_binders: Vec<Box<dyn FnOnce(&mut SetupContext, Target)>>,
}

impl<Target> ViewBuilder<Target>
where
    Target: Clone + 'static,
{
    pub fn new(creator: impl FnOnce(&mut SetupContext) -> Target + 'static) -> Self {
        Self {
            create_target: Box::new(creator),
            property_binders: vec![],
        }
    }

    pub fn bind<FrameworkType, ValueType>(
        &mut self,
        prop: &'static Prop<FrameworkType, Target, ValueType>,
        signal: impl Signal<Value = ValueType> + 'static,
    ) {
        self.property_binders
            .push(Box::new(move |ctx, target| prop.bind(ctx, target, signal)));
    }

    pub fn setup(self, ctx: &mut SetupContext) -> Target {
        let target = (self.create_target)(ctx);
        for binder in self.property_binders {
            binder(ctx, target.clone());
        }
        target
    }
}
