use crate::{
    clean_up::CleanUp,
    registry::{EffectID, RegistryRef, SignalID},
};

pub struct MountingNode {
    pub registry: RegistryRef,
    children: Vec<MountingNode>,
    cleanups: Vec<Box<dyn CleanUp>>,
    effects: Vec<EffectID>,
    signals: Vec<SignalID>,
}

impl MountingNode {
    pub fn new(registry: RegistryRef) -> Self {
        Self {
            registry,
            children: Default::default(),
            cleanups: Default::default(),
            effects: Default::default(),
            signals: Default::default(),
        }
    }
}