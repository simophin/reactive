use futures::{channel::mpsc, Future};

use crate::{
    clean_up::BoxedCleanUp,
    component::BoxedComponent,
    effect::BoxedEffect,
    react_context::{EffectID, ReactiveContext, SignalID},
    setup_context::SetupContext,
};

pub struct Node {
    pub effects: Vec<BoxedEffect>,
    pub signals: Vec<SignalID>,
    pub clean_ups: Vec<BoxedCleanUp>,
    pub children: Vec<Node>,
    pub component: BoxedComponent,
}

impl Node {
    pub fn setup_from(
        signal_change_sender: mpsc::Sender<SignalID>,
        mut component: BoxedComponent,
    ) -> Self {
        let mut ctx = SetupContext::new(signal_change_sender);
        component.setup(&mut ctx);

        let signal_change_sender = ctx.signal_change_sender;

        Self {
            effects: ctx.effects,
            signals: ctx.signals,
            clean_ups: ctx.clean_ups,
            component,
            children: ctx
                .children
                .into_iter()
                .map(|child| Self::setup_from(signal_change_sender.clone(), child))
                .collect(),
        }
    }

    pub fn mount(self, ctx: &mut ReactiveContext<impl Future>) -> MountedNode {
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
    pub fn unmount(self, ctx: &mut ReactiveContext<impl Future>) -> BoxedComponent {
        for child in self.children.into_iter() {
            child.unmount(ctx);
        }

        for effect in self.effects {
            ctx.remove_effect(effect);
        }

        self.component
    }
}
