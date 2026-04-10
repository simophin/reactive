use reactive_core::SetupContext;
use ui_core::widgets::*;

use crate::ui::button::AndroidButton;
use crate::ui::flex::AndroidFlex;
use crate::ui::image::{AndroidImage, AndroidImageCodec};
use crate::ui::label::AndroidLabel;
use crate::ui::list_view::AndroidListView;
use crate::ui::progress_indicator::AndroidProgressIndicator;
use crate::ui::slider::AndroidSlider;
use crate::ui::stack::AndroidStack;
use crate::ui::text_input::AndroidTextInput;
use crate::ui::window::AndroidWindow;

pub struct Android;

impl Platform for Android {
    type ImageCodec = AndroidImageCodec;
    type Button = AndroidButton;
    type Label = AndroidLabel;
    type Image = AndroidImage;
    type ProgressIndicator = AndroidProgressIndicator;
    type TextInput = AndroidTextInput;
    type Slider = AndroidSlider;
    type Row = AndroidFlex;
    type Column = AndroidFlex;
    type Stack = AndroidStack;
    type Window = AndroidWindow;
    type List = AndroidListView;

    fn run_app(_setup: impl FnOnce(&mut SetupContext) + 'static) {
        // On Android, the app lifecycle is managed by the Activity.
        // The Kotlin side calls nativeCreate() during Activity.onCreate(),
        // then sets up the component tree. run_app() is a no-op here
        // because the main loop is the Android Looper, not a blocking call.
    }

    fn register_back_handler(_on_back: impl FnMut() -> bool + 'static) {}
}
