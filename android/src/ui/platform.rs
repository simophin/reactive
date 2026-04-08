use ui_core::widgets::*;
use reactive_core::SetupContext;
use crate::ui::view_component::{AndroidViewComponent, NoChild};
use crate::ui::label::AndroidLabel;
use crate::ui::button::AndroidButton;
use crate::ui::flex::AndroidFlex;
use crate::ui::window::AndroidWindow;
use crate::ui::stack::AndroidStack;
use crate::ui::list_view::AndroidListView;
use crate::ui::image::AndroidImage;
use crate::ui::progress_indicator::AndroidProgressIndicator;
use crate::ui::slider::AndroidSlider;
use crate::ui::text_input::AndroidTextInput;

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

    fn run_app(setup: impl FnOnce(&mut SetupContext) + 'static) {
        // Bootstrapping logic to load dexer classes and call setup
        // will be implemented here in Phase 1/2.
    }
}
