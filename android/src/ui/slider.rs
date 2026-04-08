use ui_core::widgets::Slider;
use crate::ui::view_component::{AndroidViewBuilder, AndroidViewComponent};
use reactive_core::{Component, SetupContext, Signal};

pub struct AndroidSlider;

impl Slider for AndroidSlider {
    fn new(value: impl Signal<Value = usize>, range: impl Signal<Value = std::ops::Range<usize>>, on_change: impl Fn(usize)) -> Self {
        AndroidViewComponent(
            AndroidViewBuilder::create_no_child(
                |_ctx| {
                    todo!("Create SeekBar via JNI")
                },
                |w| todo!("Convert to AndroidView"),
            )
        )
    }
}
