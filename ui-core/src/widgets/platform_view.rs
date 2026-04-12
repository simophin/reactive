use std::any::Any;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SizeSpec {
    AtMost(usize),
    Exactly(usize),
    Unspecified,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SingleAxisMeasure {
    Independent,
    WidthForHeight(usize),
    HeightForWidth(usize),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct SingleAxisMeasureResult {
    pub min: usize,
    pub natrual: usize,
}

pub trait PlatformBaseView: Any + 'static {
    fn measure(&self, width_spec: SizeSpec, height_spec: SizeSpec) -> (usize, usize);
    fn measure_single_axis(&self, measure: SingleAxisMeasure) -> SingleAxisMeasureResult;

    fn size(&self) -> (usize, usize);

    fn request_layout(&self);

    fn as_any(&self) -> &dyn Any;
}

pub trait PlatformContainerView: PlatformBaseView {
    type BaseView: PlatformBaseView;

    fn add_child(&self, child: &Self::BaseView);
    fn update_child_at(&self, index: usize, child: &Self::BaseView);

    fn remove_child(&self, child: &Self::BaseView);
    fn remove_all_children(&self);

    fn child_at(&self, index: usize) -> Option<&Self::BaseView>;

    fn child_count(&self) -> usize;

    fn place_child(&self, child_index: usize, pos: (usize, usize), size: (usize, usize));
}

pub struct DoubleAxisMeasureRequest(SizeSpec, SizeSpec);

pub trait CustomLayoutOperation {
    type BaseView: PlatformContainerView;

    fn on_measure(
        &self,
        view: &Self::BaseView,
        width: SizeSpec,
        height: SizeSpec,
    ) -> (usize, usize);

    fn on_measure_single(
        &self,
        view: &Self::BaseView,
        measure: SingleAxisMeasure,
    ) -> SingleAxisMeasureResult;

    fn on_layout(&self, view: &Self::BaseView, size: (usize, usize));
}
