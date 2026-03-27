use objc2::rc::Retained;

pub type ViewBuilder<ViewType> = ui_core::ViewBuilder<Retained<ViewType>>;
