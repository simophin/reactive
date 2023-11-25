use reactive_core::Signal;

use super::view::{AndroidView, AndroidViewBuilder, AndroidViewBuilderError};

pub struct TextViewBuilder {
    builder: AndroidViewBuilder,
}

impl Default for TextViewBuilder {
    fn default() -> Self {
        Self {
            builder: AndroidViewBuilder::default()
                .auto_adopt_child(false)
                .class_name("android.widget.TextView"),
        }
    }
}

impl TextViewBuilder {
    pub fn text(self, text: impl Signal<Value = String>) -> Self {
        Self {
            builder: self
                .builder
                .property("setText", "(Ljava/lang/String;)V", text),
        }
    }

    pub fn build(self) -> Result<AndroidView, AndroidViewBuilderError> {
        self.builder.build()
    }
}
