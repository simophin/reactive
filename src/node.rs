use crate::{
    clean_up::BoxedCleanUp,
    component::BoxedComponent,
    effect::BoxedEffect,
    react_context::ReactiveContext,
    registry::{EffectID, SignalID},
    setup::SetupContext,
};

pub struct Node {
    pub effects: Vec<BoxedEffect>,
    pub signals: Vec<SignalID>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<Node>,
    pub component: BoxedComponent,
}

impl Node {
    pub fn setup_from(mut component: BoxedComponent) -> Self {
        SetupContext::set_current(Some(Default::default()));

        component.setup();

        let SetupContext {
            effects,
            signals,
            clean_ups,
            children,
        } = SetupContext::set_current(None).expect("To have setup context set before");

        Self {
            effects,
            signals,
            clean_ups,
            component,
            children: children.into_iter().map(Self::setup_from).collect(),
        }
    }

    pub fn mount(self, ctx: &mut ReactiveContext) -> MountedNode {
        let children = self.children.into_iter().map(|c| c.mount(ctx)).collect();

        let mut effects = vec![];
        for effect in self.effects {
            let id = ctx.new_effect(effect);
            effects.push(id);
            ctx.schedule_effect_run(id);
        }

        MountedNode {
            effects,
            signals: self.signals,
            clean_ups: self.clean_ups,
            children,
            component: self.component,
        }
    }
}

pub struct MountedNode {
    pub effects: Vec<EffectID>,
    pub signals: Vec<SignalID>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<MountedNode>,
    pub component: BoxedComponent,
}

impl MountedNode {
    pub fn unmount(self, ctx: &mut ReactiveContext) -> BoxedComponent {
        for child in self.children.into_iter() {
            child.unmount(ctx);
        }

        for effect in self.effects {
            ctx.remove_effect(effect);
        }

        self.component
    }
}
