use appkit::platform::AppKit;
use appkit::run_app;
use appkit::text_view::AppKitText;
use reactive_core::{SetupContext, Signal, SignalExt};
use ui_core::layout::{CrossAxisAlignment, EdgeInsets, Padding, SizedBox};
use ui_core::widgets::{
    Button, Column, Label, Platform, TextChange, TextInput, TextInputState, Window,
};

// ---------------------------------------------------------------------------
// Demo — three text inputs that showcase different control modes:
//
//  1. Echo     — uncontrolled: signal always accepts the change as-is, and a
//                label below mirrors the current value reactively.
//
//  2. Digits   — filtered: the on_change callback rejects any replacement that
//                contains non-digit characters.  Because the signal is not
//                updated, shouldChangeTextInRange returns NO and NSTextKit
//                discards the edit before it ever reaches the backing store.
//                No flicker, no guard variables.
//
//  3. Reset    — demonstrates programmatic signal update: clicking the button
//                sets the signal value directly, which the effect in TextInput
//                pushes into NSTextStorage via set_text().
// ---------------------------------------------------------------------------

fn demo<P>(ctx: &mut SetupContext)
where
    P: Platform,
    P::TextInput: TextInput<PlatformTextType = AppKitText>,
{
    let echo = ctx.create_signal(TextInputState {
        text: AppKitText::from("Hello, world!"),
        selection: 0..13,
    });

    let digits = ctx.create_signal(TextInputState {
        text: AppKitText::from(""),
        selection: 0..0,
    });

    ctx.child(Padding {
        insets: EdgeInsets::all(24),
        child: P::Column::new()
            .spacing(10usize)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            // ── Header ──────────────────────────────────────────────────────
            .child(P::Label::new("Text Input Widget Demo").font_size(20.0))
            // ── Echo ────────────────────────────────────────────────────────
            .child(P::Label::new("Echo (uncontrolled — accepts all input):"))
            .child(SizedBox::height(28usize).child(P::TextInput::new(
                echo.clone(),
                {
                    let echo = echo.clone();
                    move |change| {
                        let mut s = echo.read();
                        s.apply_change(&change);
                        echo.update_if_changes(s);
                    }
                },
            )))
            .child(P::Label::new(
                echo.clone().map_value(|s| format!("→ \"{}\"  ({} chars)", s.text, s.text.to_string().len())),
            ))
            // ── Digits-only ──────────────────────────────────────────────────
            .child(P::Label::new(
                "Digits only (non-digit keystrokes are rejected before the backing store is touched):",
            ))
            .child(SizedBox::height(28usize).child(P::TextInput::new(
                digits.clone(),
                {
                    let digits = digits.clone();
                    move |change| {
                        // Reject replacements that contain non-digit characters.
                        // Returning without updating the signal causes
                        // shouldChangeTextInRange: to return NO — NSTextKit
                        // discards the edit entirely.
                        if let TextChange::Replacement { with, .. } = &change {
                            if !with.to_string().chars().all(|c| c.is_ascii_digit()) {
                                return;
                            }
                        }
                        let mut s = digits.read();
                        s.apply_change(&change);
                        digits.update_if_changes(s);
                    }
                },
            )))
            .child(P::Label::new(
                digits.clone().map_value(|s| format!("→ \"{}\"  ({} digits)", s.text, s.text.to_string().len())),
            ))
            // ── Reset ────────────────────────────────────────────────────────
            .child(SizedBox::height(8usize).child(()))
            .child(SizedBox::height(32usize).child(P::Button::new(
                "Reset both fields",
                {
                    let echo = echo.clone();
                    let digits = digits.clone();
                    move || {
                        echo.update_if_changes(TextInputState {
                            text: AppKitText::from("Hello, world!"),
                            selection: 0..13,
                        });
                        digits.update_if_changes(TextInputState {
                            text: AppKitText::from(""),
                            selection: 0..0,
                        });
                    }
                },
            ))),
    });
}

fn main() {
    run_app(|ctx| {
        ctx.child(appkit::window::Window::new(
            "Text Input Demo",
            demo::<AppKit>,
            480.0,
            380.0,
        ));
    });
}
