use rg3d::{
    core::pool::Handle,
    gui::{
        UINode,
        UserInterface,
        window::{WindowBuilder, WindowTitle},
        widget::WidgetBuilder,
        grid::GridBuilder,
        Builder,
        grid::{Row, Column},
        text::TextBuilder,
        scroll_bar::ScrollBarBuilder
    }
};

pub struct MatchMenu {
    window: Handle<UINode>
}

impl MatchMenu {
    pub fn new(ui: &mut UserInterface) -> Self {
        let window = WindowBuilder::new(WidgetBuilder::new()
            .with_width(500.0))
            .with_title(WindowTitle::Text("Match Options"))
            .open(false)
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(0)
                    .on_column(0))
                    .with_text("Time Limit")
                    .build(ui))
                .with_child(ScrollBarBuilder::new(WidgetBuilder::new()
                    .on_row(0)
                    .on_column(1))
                    .build(ui)))
                .add_column(Column::strict(200.0))
                .add_column(Column::stretch())
                .add_row(Row::strict(30.0))
                .add_row(Row::stretch())
                .build(ui))
            .build(ui);
        Self {
            window
        }
    }
}