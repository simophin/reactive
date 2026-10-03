//! iOS backend built on UIKit.
//!
//! Rust owns real `UIView`s and signals call their setters directly. The app
//! uses the scene-based lifecycle with two Rust-defined Objective-C classes,
//! `ReactiveAppDelegate` and `ReactiveSceneDelegate`; see [`run_app`].

mod app;
mod ui;

pub use app::{run_app, run_app_reconnectable};
pub use ui::*;
