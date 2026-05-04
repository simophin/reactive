use reactive_core::SetupContext;
use ui_core::widgets::*;

use crate::android::ui::button::AndroidButton;
use crate::android::ui::flex::AndroidFlex;
use crate::android::ui::image::{AndroidImage, AndroidImageCodec};
use crate::android::ui::label::AndroidLabel;
use crate::android::ui::list_view::AndroidListView;
use crate::android::ui::progress_indicator::AndroidProgressIndicator;
use crate::android::ui::slider::AndroidSlider;
use crate::android::ui::stack::AndroidStack;
use crate::android::ui::text_input::AndroidTextInput;
use crate::android::ui::window::AndroidWindow;

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
