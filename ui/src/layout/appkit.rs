use reactive_core::Component;

pub type Row = appkit::Stack;
pub type Column = appkit::Stack;

impl super::RowComponent for Row {
    fn new() -> Self {
        appkit::Stack::new_horizontal_stack()
    }

    fn child(self, c: impl Component + 'static) -> Self {
        Row::child(self, c)
    }
}

impl super::ColumnComponent for Column {
    fn new() -> Self {
        appkit::Stack::new_vertical_stack()
    }

    fn child(self, c: impl Component + 'static) -> Self {
        Column::child(self, c)
    }
}
