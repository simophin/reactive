use crate::layout::{CrossAxisAlignment, EdgeInsets};
use crate::widgets::{
    CommonModifiers, CustomLayoutOperation, Modifier, ModifierKey, NATIVE_VIEW_REGISTRY,
    NativeView, NativeViewRegistry, Platform, PlatformBaseView, PlatformContainerView,
    SingleAxisMeasure, SingleAxisMeasureResult, SizeSpec,
};
use reactive_core::{BoxedComponent, Component, IntoSignal, ReactiveScope, SetupContext, Signal};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

static WEIGHT_KEY: ModifierKey<f64> = ModifierKey::with_merger(|_old_signal, new_value| new_value);

pub struct FlexScope {
    pub weight: &'static ModifierKey<f64>,
}

impl FlexScope {
    fn new() -> Self {
        Self {
            weight: &WEIGHT_KEY,
        }
    }
}

pub(crate) struct Flex<P, S, A> {
    vertical: bool,
    spacing: S,
    cross_axis_alignment: A,
    children: Vec<Box<dyn FnOnce(&FlexScope) -> BoxedComponent>>,
    modifier: Modifier,
    _platform_marker: PhantomData<P>,
}

impl<P, S, A> Flex<P, S, A> {
    pub fn child<C: Component + 'static>(
        mut self,
        child: impl FnOnce(&FlexScope) -> C + 'static,
    ) -> Self {
        self.children
            .push(Box::new(move |scope| Box::new(child(scope))));
        self
    }
}

struct FlexChildData<PlatformView> {
    view: Option<PlatformView>,
    modifier: Modifier,
}

impl<PV> Default for FlexChildData<PV> {
    fn default() -> Self {
        Self {
            view: Default::default(),
            modifier: Default::default(),
        }
    }
}

struct FlexNativeViewRegistry<PlatformContainerView, PlatformView> {
    parent: PlatformContainerView,
    index_in_parent: usize,
    data: Rc<RefCell<FlexChildData<PlatformView>>>,
}

impl<PC, PV> NativeViewRegistry for FlexNativeViewRegistry<PC, PV>
where
    PV: PlatformBaseView + Clone + Eq,
    PC: PlatformContainerView<BaseView = PV>,
{
    fn update_platform_view(&self, view: &dyn PlatformBaseView, modifier: Modifier) {
        let view = view.as_any().downcast_ref::<PV>().unwrap();
        if self.parent.child_count() <= self.index_in_parent {
            self.parent.add_child(view);
        } else {
            self.parent.update_child_at(self.index_in_parent, view);
        }

        let mut data = self.data.borrow_mut();
        data.view.replace(view.clone());
        data.modifier = modifier;
        drop(data);

        self.parent.request_layout();
    }

    fn clear_platform_view(&self, view: &dyn PlatformBaseView) {
        let view = view.as_any().downcast_ref::<PV>().unwrap();
        self.parent.remove_child(view);

        match self.data.borrow().view.as_ref() {
            Some(child) if child == view => {
                let _ = self.data.borrow_mut().view.take();
            }

            _ => {}
        }
    }
}

struct FlexChild<PC, PV> {
    child: BoxedComponent,
    registry: Rc<dyn NativeViewRegistry>,
    parent: PC,
    data: Rc<RefCell<FlexChildData<PV>>>,
}

impl<PC, PV> Component for FlexChild<PC, PV>
where
    PC: PlatformContainerView,
    PV: 'static,
{
    fn setup(self: Box<Self>, ctx: &mut SetupContext) {
        let Self {
            child,
            registry,
            parent,
            data,
        } = *self;

        ctx.set_context(&NATIVE_VIEW_REGISTRY, registry.into_signal());
        ctx.boxed_child(child);

        ctx.create_effect(move |_, last: Option<(EdgeInsets, f64)>| {
            let next = {
                let data = data.borrow();
                (
                    data.modifier.get_paddings().read().unwrap_or_default(),
                    data.modifier
                        .get(&WEIGHT_KEY)
                        .map(|signal| signal.read())
                        .unwrap_or_default(),
                )
            };

            if last.is_some() && last != Some(next) {
                parent.request_layout();
            }

            next
        });
    }
}

struct FlexLayout<PV, PC, S, A> {
    children: Rc<[Rc<RefCell<FlexChildData<PV>>>]>,
    spacing: S,
    cross_axis_alignment: A,
    vertical: bool,
    _marker: PhantomData<fn() -> PC>,
}

impl<PV, PC, S, A> FlexLayout<PV, PC, S, A>
where
    PV: PlatformBaseView + Clone,
{
    fn main_axis<T>(&self, x: T, y: T) -> T {
        if self.vertical { y } else { x }
    }

    fn cross_axis<T>(&self, x: T, y: T) -> T {
        if self.vertical { x } else { y }
    }

    fn child_padding_and_weight(&self, index: usize) -> (EdgeInsets, f64) {
        let data = self.children[index].borrow();
        (
            data.modifier.get_paddings().read().unwrap_or_default(),
            data.modifier.get(&WEIGHT_KEY).read().unwrap_or_default(),
        )
    }

    fn child_view(&self, index: usize) -> Option<PV> {
        self.children[index].borrow().view.clone()
    }

    fn measure_child(
        &self,
        index: usize,
        width: SizeSpec,
        height: SizeSpec,
    ) -> Option<((usize, usize), EdgeInsets, f64)> {
        let child_view = self.child_view(index)?;
        let (padding, weight) = self.child_padding_and_weight(index);
        let measured = child_view.measure(
            shrink_size_spec(width, padding.left + padding.right),
            shrink_size_spec(height, padding.top + padding.bottom),
        );

        Some((measured, padding, weight))
    }

    fn padding_main(&self, padding: EdgeInsets) -> usize {
        self.main_axis(padding.left + padding.right, padding.top + padding.bottom)
    }

    fn padding_cross(&self, padding: EdgeInsets) -> usize {
        self.cross_axis(padding.left + padding.right, padding.top + padding.bottom)
    }
}

impl<PV, PC, S, A> CustomLayoutOperation for FlexLayout<PV, PC, S, A>
where
    PV: PlatformBaseView + Clone,
    S: Signal<Value = usize> + 'static,
    A: Signal<Value = CrossAxisAlignment> + 'static,
    PC: PlatformContainerView<BaseView = PV>,
{
    type BaseView = PC;

    fn on_measure(
        &self,
        _view: &Self::BaseView,
        width: SizeSpec,
        height: SizeSpec,
    ) -> (usize, usize) {
        let spacing = self.spacing.read();
        let cross_axis_alignment = self.cross_axis_alignment.read();
        let child_count = self.children.len();

        let cross_axis_spec = if self.vertical { width } else { height };
        let cross_measure_spec = match cross_axis_alignment {
            CrossAxisAlignment::Stretch => exact_if_bounded(cross_axis_spec),
            CrossAxisAlignment::Start | CrossAxisAlignment::Center | CrossAxisAlignment::End => {
                at_most_if_bounded(cross_axis_spec)
            }
        };

        let mut main_axis = 0usize;
        let mut cross_axis = 0usize;

        for idx in 0..child_count {
            let Some((measured, padding, _weight)) = self.measure_child(
                idx,
                self.cross_axis(cross_measure_spec, SizeSpec::Unspecified),
                self.main_axis(SizeSpec::Unspecified, cross_measure_spec),
            ) else {
                continue;
            };

            let outer_width = measured.0 + padding.left + padding.right;
            let outer_height = measured.1 + padding.top + padding.bottom;

            main_axis += self.main_axis(outer_width, outer_height);
            cross_axis = cross_axis.max(self.cross_axis(outer_width, outer_height));
        }

        if child_count > 1 {
            main_axis += spacing * (child_count - 1);
        }

        (
            resolve_size_spec(width, self.cross_axis(cross_axis, main_axis)),
            resolve_size_spec(height, self.main_axis(cross_axis, main_axis)),
        )
    }

    fn on_measure_single(
        &self,
        _view: &Self::BaseView,
        measure: SingleAxisMeasure,
    ) -> SingleAxisMeasureResult {
        let spacing = self.spacing.read();
        let child_count = self.children.len();

        let add_spacing = |value: usize| {
            if child_count > 1 {
                value + spacing * (child_count - 1)
            } else {
                value
            }
        };

        match measure {
            SingleAxisMeasure::Independent => {
                let mut min = 0usize;
                let mut natural = 0usize;

                for idx in 0..child_count {
                    let Some(child_view) = self.child_view(idx) else {
                        continue;
                    };
                    let (padding, _) = self.child_padding_and_weight(idx);
                    let child = child_view.measure_single_axis(SingleAxisMeasure::Independent);
                    let padded_min = child.min + self.padding_cross(padding);
                    let padded_natural = child.natrual + self.padding_cross(padding);

                    min = min.max(padded_min);
                    natural = natural.max(padded_natural);
                }

                SingleAxisMeasureResult {
                    min,
                    natrual: natural,
                }
            }
            SingleAxisMeasure::WidthForHeight(height) if !self.vertical => {
                let mut min = 0usize;
                let mut natural = 0usize;

                for idx in 0..child_count {
                    let Some(child_view) = self.child_view(idx) else {
                        continue;
                    };
                    let (padding, _) = self.child_padding_and_weight(idx);
                    let child = child_view.measure_single_axis(SingleAxisMeasure::WidthForHeight(
                        height.saturating_sub(padding.top + padding.bottom),
                    ));
                    min += child.min + self.padding_main(padding);
                    natural += child.natrual + self.padding_main(padding);
                }

                SingleAxisMeasureResult {
                    min: add_spacing(min),
                    natrual: add_spacing(natural),
                }
            }
            SingleAxisMeasure::HeightForWidth(width) if self.vertical => {
                let mut min = 0usize;
                let mut natural = 0usize;

                for idx in 0..child_count {
                    let Some(child_view) = self.child_view(idx) else {
                        continue;
                    };
                    let (padding, _) = self.child_padding_and_weight(idx);
                    let child = child_view.measure_single_axis(SingleAxisMeasure::HeightForWidth(
                        width.saturating_sub(padding.left + padding.right),
                    ));
                    min += child.min + self.padding_main(padding);
                    natural += child.natrual + self.padding_main(padding);
                }

                SingleAxisMeasureResult {
                    min: add_spacing(min),
                    natrual: add_spacing(natural),
                }
            }
            SingleAxisMeasure::WidthForHeight(_) | SingleAxisMeasure::HeightForWidth(_) => {
                self.on_measure_single(_view, SingleAxisMeasure::Independent)
            }
        }
    }

    fn on_layout(&self, view: &Self::BaseView, size: (usize, usize)) {
        let spacing = self.spacing.read();
        let cross_axis_alignment = self.cross_axis_alignment.read();
        let child_count = self.children.len();

        if child_count == 0 {
            return;
        }

        let available_main = self.main_axis(size.0, size.1);
        let available_cross = self.cross_axis(size.0, size.1);
        let cross_measure_spec = match cross_axis_alignment {
            CrossAxisAlignment::Stretch => SizeSpec::Exactly(available_cross),
            CrossAxisAlignment::Start | CrossAxisAlignment::Center | CrossAxisAlignment::End => {
                SizeSpec::AtMost(available_cross)
            }
        };

        let mut main_sizes = vec![0usize; child_count];
        let mut weights = vec![0.0f64; child_count];
        let mut paddings = vec![EdgeInsets::default(); child_count];
        let mut fixed_main = 0usize;
        let mut total_weight = 0.0f64;

        for idx in 0..child_count {
            let Some((measured, padding, weight)) = self.measure_child(
                idx,
                self.cross_axis(cross_measure_spec, SizeSpec::Unspecified),
                self.main_axis(SizeSpec::Unspecified, cross_measure_spec),
            ) else {
                continue;
            };

            let outer_width = measured.0 + padding.left + padding.right;
            let outer_height = measured.1 + padding.top + padding.bottom;
            let natural_main = self.main_axis(outer_width, outer_height);

            paddings[idx] = padding;
            weights[idx] = weight;

            if weight > 0.0 {
                total_weight += weight;
                main_sizes[idx] = natural_main;
            } else {
                fixed_main += natural_main;
                main_sizes[idx] = natural_main;
            }
        }

        let spacing_total = spacing.saturating_mul(child_count.saturating_sub(1));
        let remaining = available_main.saturating_sub(fixed_main + spacing_total);
        let mut assigned_weighted = 0usize;
        let mut seen_weight = 0.0f64;

        for idx in 0..child_count {
            let weight = weights[idx];
            if weight <= 0.0 {
                continue;
            }

            let allocated = if seen_weight + weight >= total_weight {
                remaining.saturating_sub(assigned_weighted)
            } else {
                ((remaining as f64) * (weight / total_weight)).round() as usize
            };

            main_sizes[idx] = main_sizes[idx].max(allocated);
            assigned_weighted += allocated;
            seen_weight += weight;
        }

        let mut cursor = 0usize;
        for idx in 0..child_count {
            let Some(child_view) = self.child_view(idx) else {
                continue;
            };

            let padding = paddings[idx];
            let main_size = main_sizes[idx];
            let inner_main = main_size.saturating_sub(
                self.main_axis(padding.left + padding.right, padding.top + padding.bottom),
            );

            let outer_cross = match cross_axis_alignment {
                CrossAxisAlignment::Stretch => available_cross,
                CrossAxisAlignment::Start
                | CrossAxisAlignment::Center
                | CrossAxisAlignment::End => {
                    let measured = child_view.measure(
                        self.cross_axis(
                            at_most_if_bounded(SizeSpec::AtMost(available_cross)),
                            SizeSpec::Exactly(inner_main),
                        ),
                        self.main_axis(
                            SizeSpec::Exactly(inner_main),
                            at_most_if_bounded(SizeSpec::AtMost(available_cross)),
                        ),
                    );

                    self.cross_axis(
                        measured.0 + padding.left + padding.right,
                        measured.1 + padding.top + padding.bottom,
                    )
                }
            };

            let cross_offset = match cross_axis_alignment {
                CrossAxisAlignment::Stretch | CrossAxisAlignment::Start => 0,
                CrossAxisAlignment::Center => available_cross.saturating_sub(outer_cross) / 2,
                CrossAxisAlignment::End => available_cross.saturating_sub(outer_cross),
            };

            let outer_x = self.cross_axis(cross_offset, cursor);
            let outer_y = self.main_axis(cross_offset, cursor);
            let inner_x = outer_x + padding.left;
            let inner_y = outer_y + padding.top;
            let inner_cross = outer_cross.saturating_sub(
                self.cross_axis(padding.left + padding.right, padding.top + padding.bottom),
            );

            view.place_child(
                idx,
                (inner_x, inner_y),
                self.cross_axis((inner_cross, inner_main), (inner_main, inner_cross)),
            );

            cursor += main_size + if idx + 1 < child_count { spacing } else { 0 };
        }
    }
}

impl<P, S, A> Component for Flex<P, S, A>
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
            children,
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
                    P::new_custom_layout(FlexLayout::<P::View, P::ContainerView, _, _> {
                        children,
                        vertical,
                        spacing,
                        cross_axis_alignment,
                        _marker: PhantomData,
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

        let flex_scope = FlexScope::new();

        for (idx, (child, child_data)) in children
            .into_iter()
            .zip(children_data.into_iter().cloned())
            .enumerate()
        {
            ctx.child(FlexChild::<P::ContainerView, P::View> {
                child: child(&flex_scope),
                registry: Rc::new(FlexNativeViewRegistry::<P::ContainerView, P::View> {
                    parent: native_view.clone(),
                    data: child_data.clone(),
                    index_in_parent: idx,
                }),
                parent: native_view.clone(),
                data: child_data,
            });
        }
    }
}

fn exact_if_bounded(spec: SizeSpec) -> SizeSpec {
    match spec {
        SizeSpec::Exactly(size) | SizeSpec::AtMost(size) => SizeSpec::Exactly(size),
        SizeSpec::Unspecified => SizeSpec::Unspecified,
    }
}

fn at_most_if_bounded(spec: SizeSpec) -> SizeSpec {
    match spec {
        SizeSpec::Exactly(size) | SizeSpec::AtMost(size) => SizeSpec::AtMost(size),
        SizeSpec::Unspecified => SizeSpec::Unspecified,
    }
}

fn shrink_size_spec(spec: SizeSpec, by: usize) -> SizeSpec {
    match spec {
        SizeSpec::AtMost(size) => SizeSpec::AtMost(size.saturating_sub(by)),
        SizeSpec::Exactly(size) => SizeSpec::Exactly(size.saturating_sub(by)),
        SizeSpec::Unspecified => SizeSpec::Unspecified,
    }
}

fn resolve_size_spec(spec: SizeSpec, natural: usize) -> usize {
    match spec {
        SizeSpec::Exactly(size) => size,
        SizeSpec::AtMost(size) => natural.min(size),
        SizeSpec::Unspecified => natural,
    }
}
