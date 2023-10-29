use std::cell::RefCell;
use std::rc::Rc;

use crate::component::BoxedComponent;
use crate::node::Node;
use crate::registry::Registry;

pub struct RenderContext;

impl RenderContext {
    pub fn new(root: BoxedComponent) -> Initialized {
        Initialized(root)
    }
}

pub struct Initialized(BoxedComponent);

impl Initialized {
    pub fn mount(self) -> Mounted {
        let mut component = self.0;

        Registry::set_current(Some(Registry::new()));
        let node = Self::mount_component(&mut component);
        let registry = Registry::set_current(None).expect("Registry to have been set before");

        registry.borrow_mut().call_all_effects();

        Mounted {
            component,
            node,
            registry,
        }
    }

    fn mount_component(component: &mut BoxedComponent) -> Node {
        Node::set_current(Some(Node::new(Registry::current())));

        let mut children = Default::default();
        component.render(&mut children);

        let mut node = Node::set_current(None).expect("Node should have set before");

        for mut child in children {
            node.append_child(Self::mount_component(&mut child));
        }

        node
    }
}

pub struct Mounted {
    component: BoxedComponent,
    node: Node,
    registry: Rc<RefCell<Registry>>,
}

impl Mounted {
    pub fn unmount(self) -> Initialized {
        Initialized(self.component)
    }
}
