use crate::layout::{
    BOX_MODIFIERS, BoxModifierChain, CrossAxisAlignment, FLEX_PARENT_DATA, FlexParentData,
};
use crate::widgets::{
    CustomLayoutOperation, Modifier, NATIVE_VIEW_REGISTRY, NativeView, NativeViewRegistry,
    Platform, PlatformBaseView, PlatformContainerView, SingleAxisMeasure, SingleAxisMeasureResult,
    SizeSpec,
};
use crate::{ChildStrategy, MultipleChildren};
use reactive_core::{BoxedComponent, Component, IntoSignal, ReactiveScope, SetupContext, Signal};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

pub(crate) struct Flex<P, C, S, A> {
    vertical: bool,
    spacing: S,
    cross_axis_alignment: A,
    children: C,
    modifier: Modifier,
    _platform_marker: PhantomData<P>,
}

struct FlexChildData<P: Platform> {
    view: Option<P::View>,
    modifier: Modifier,
}

impl<P: Platform> Default for FlexChildData<P> {
    fn default() -> Self {
        Self {
            view: Default::default(),
            modifier: Default::default(),
        }
    }
}

struct FlexNativeViewRegistry<P: Platform> {
    parent: P::ContainerView,
    index_in_parent: usize,
    data: Rc<RefCell<FlexChildData<P>>>,
}

impl<P> NativeViewRegistry for FlexNativeViewRegistry<P>
where
    P: Platform,
{
    fn update_platform_view(&self, view: &dyn PlatformBaseView, modifier: Modifier) {
        let view = view.as_any().downcast_ref::<P::View>().unwrap();
        if self.parent.child_count() <= self.index_in_parent {
            self.parent.add_child(view);
        } else {
            self.parent.update_child_at(self.index_in_parent, view);
        }

        let mut data = self.data.borrow_mut();
        data.view.replace(view.clone());
        data.modifier = modifier;
    }

    fn clear_platform_view(&self, view: &dyn PlatformBaseView) {
        let view = view.as_any().downcast_ref::<P::View>().unwrap();
        self.parent.remove_child(view);

        match self.data.borrow().view.as_ref() {
            Some(child) if child == view => {
                let _ = self.data.borrow_mut().view.take();
            }

            _ => {}
        }
    }
}

struct FlexChild {
    child: BoxedComponent,
    registry: Rc<dyn NativeViewRegistry>,
}

impl Component for FlexChild {
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        ctx.set_context(&NATIVE_VIEW_REGISTRY, self.registry.into_signal());
        ctx.boxed_child(self.child);
    }
}

struct FlexLayout<P: Platform, S, A> {
    children: Rc<[Rc<RefCell<FlexChildData<P>>>]>,
    spacing: S,
    cross_axis_alignment: A,
    vertical: bool,
}

impl<P: Platform, S, A> FlexLayout<P, S, A> {
    fn main_axis<T>(&self, x: T, y: T) -> T {
        if self.vertical { y } else { x }
    }

    fn cross_axis<T>(&self, x: T, y: T) -> T {
        if self.vertical { x } else { y }
    }
}

impl<P, S, A> CustomLayoutOperation for FlexLayout<P, S, A>
where
    P: Platform,
    S: Signal<Value = usize> + 'static,
    A: Signal<Value = CrossAxisAlignment> + 'static,
{
    type BaseView = P::ContainerView;

    fn on_measure(
        &self,
        view: &Self::BaseView,
        width: SizeSpec,
        height: SizeSpec,
    ) -> (usize, usize) {
        let (main_axis_spec, cross_axis_spec) = if self.vertical {
            (height, width)
        } else {
            (width, height)
        };

        todo!()
    }

    fn on_measure_single(
        &self,
        view: &Self::BaseView,
        measure: SingleAxisMeasure,
    ) -> SingleAxisMeasureResult {
        todo!()
    }

    fn on_layout(&self, view: &Self::BaseView, size: (usize, usize)) {
        todo!()
    }
}

impl<P, S, A> Component for Flex<P, MultipleChildren, S, A>
where
    P: Platform,
    S: Signal<Value = usize> + 'static,
    A: Signal<Value = CrossAxisAlignment> + 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            vertical,
            spacing,
            cross_axis_alignment,
            children: MultipleChildren(children),
            modifier,
            ..
        } = *self;

        let spacing: Rc<dyn Signal<Value = usize>> = Rc::new(spacing);
        let cross_axis_alignment: Rc<dyn Signal<Value = CrossAxisAlignment>> =
            Rc::new(cross_axis_alignment);

        let children_data: Rc<[_]> = vec![Default::default(); children.len()].into();

        // We are native view as well so we should be part of the train...
        let native_view = {
            let children = Rc::clone(&children_data);
            let spacing = Rc::clone(&spacing);
            let cross_axis_alignment = Rc::clone(&cross_axis_alignment);
            NativeView {
                create: move || {
                    P::new_custom_layout(FlexLayout {
                        children,
                        vertical,
                        spacing,
                        cross_axis_alignment,
                    })
                },
                on_update: move |_view: &mut P::ContainerView, _scope: &ReactiveScope| {},
                modifier,
            }
            .setup_in_component(ctx)
        };

        // Trigger layout if spacing/alignment changes
        {
            let native_view = native_view.clone();
            ctx.create_effect(move |_, last| {
                let _ = spacing.read();
                let _ = cross_axis_alignment.read();

                if last.is_some() {
                    native_view.request_layout();
                }

                ()
            });
        }

        // Reset wrapper context
        ctx.set_context(&BOX_MODIFIERS, BoxModifierChain::default().into_signal());
        ctx.set_context(&FLEX_PARENT_DATA, FlexParentData::default().into_signal());

        for (idx, (child, child_data)) in children
            .into_iter()
            .zip(children_data.into_iter().cloned())
            .enumerate()
        {
            ctx.child(FlexChild {
                child,
                registry: Rc::new(FlexNativeViewRegistry::<P> {
                    parent: native_view.clone(),
                    data: child_data,
                    index_in_parent: idx,
                }),
            });
        }
    }
}
