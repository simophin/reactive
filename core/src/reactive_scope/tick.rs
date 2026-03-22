use crate::component_scope::{ComponentId, ComponentScope, Effect};
use std::task::{Context, Poll};

use super::ReactiveScope;

impl ReactiveScope {
    pub(crate) fn traverse_tree_depth_last(
        &mut self,
        start: ComponentId,
        f: &mut impl FnMut(ComponentId, &mut ComponentScope),
    ) {
        let Some(scope) = self.components.get_mut(start) else {
            return;
        };

        f(start, scope);

        let children = std::mem::take(&mut scope.children);
        for child in &children {
            self.traverse_tree_depth_last(*child, f);
        }

        if let Some(scope) = self.components.get_mut(start) {
            scope.children = children;
        }
    }

    pub fn tick(&mut self, future_ctx: &mut Context) {
        struct EffectUpdate {
            component_id: ComponentId,
            effect: Effect,
            dirty: bool,
        }

        self.dirty_signals.set_waker(future_ctx.waker().clone());

        let mut updates = Vec::new();
        let root = std::mem::take(&mut self.root);
        let dirty_signal_set = self.dirty_signals.take();

        for component in &root {
            self.traverse_tree_depth_last(*component, &mut |component_id, c| {
                for effect in std::mem::take(&mut c.effects) {
                    let dirty = effect
                        .effect_state
                        .signal_accessed
                        .intersects(&dirty_signal_set);

                    if dirty || effect.effect_state.pending_future.is_some() {
                        updates.push(EffectUpdate {
                            component_id,
                            effect,
                            dirty,
                        });
                    } else {
                        c.effects.push(effect);
                    }
                }
            });
        }

        self.root = root;

        for mut update in updates {
            if update.dirty {
                update.effect.effect_state = (update.effect.effect_fn)(self);
            }

            if let Some(mut fut) = std::mem::take(&mut update.effect.effect_state.pending_future) {
                if let Poll::Pending = fut.as_mut().poll(future_ctx) {
                    update.effect.effect_state.pending_future.replace(fut);
                }
            }

            if let Some(c) = self.components.get_mut(update.component_id) {
                c.effects.push(update.effect);
            }
        }
    }
}
