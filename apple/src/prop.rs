use objc2::rc::Retained;

pub type Prop<FrameworkType, ViewType, ValueType> =
    ui_core::Prop<FrameworkType, Retained<ViewType>, ValueType>;
