use std::{cell::RefCell, rc::Rc};

use crate::{component::Component, effect::create_effect, node_ref::NodeRef};

pub struct Show<F, CS, CF> {
    test: Rc<RefCell<F>>,
    success: Rc<RefCell<CS>>,
    fail: Rc<RefCell<CF>>,
}

impl<F, CS, CF> Show<F, CS, CF> {
    pub fn new(test: F, success: CS, fail: CF) -> Self {
        Self {
            test: Rc::new(RefCell::new(test)),
            success: Rc::new(RefCell::new(success)),
            fail: Rc::new(RefCell::new(fail)),
        }
    }
}

impl<F, CS, CF> Component for Show<F, CS, CF>
where
    F: FnMut() -> bool + 'static,
    CS: Component,
    CF: Component,
{
    fn setup(&mut self, _output: &mut Vec<Box<dyn Component>>) {
        let test = self.test.clone();
        let success = self.success.clone();
        let fail = self.fail.clone();
        create_effect(move || {
            let mut children = Vec::with_capacity(1);

            if test.borrow_mut()() {
                success.borrow_mut().setup(&mut children);
            } else {
                fail.borrow_mut().setup(&mut children);
            }

            NodeRef::with_current(move |node| {
                let node = node.expect("To have current node");
                node.remove_all_children();
                for child in children {
                    node.append_child(child);
                }
            })
        });
    }
}
