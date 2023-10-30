use crate::component::BoxedComponent;
use crate::node::NodeRef;

pub struct RenderContext;

impl RenderContext {
    pub fn new(root: BoxedComponent) -> Initialized {
        Initialized(NodeRef::new(Default::default(), root))
    }
}

pub struct Initialized(NodeRef);

impl Initialized {
    pub fn mount(self) -> Mounted {
        self.0.mount();
        Mounted(self.0)
    }
}

pub struct Mounted(NodeRef);

impl Mounted {
    pub fn unmount(self) -> Initialized {
        self.0.unmount();
        Initialized(self.0)
    }
}
