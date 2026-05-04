use reactive_core::ReactiveScope;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct AndroidScope {
    pub scope: ReactiveScope,
    pub tick_scheduled: Arc<AtomicBool>,
}
