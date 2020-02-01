use rg3d::gui::{
    UINode, UserInterface,
    window::{WindowBuilder, WindowTitle},
    widget::WidgetBuilder,
    grid::GridBuilder
};
use rg3d::core::pool::Handle;
use rg3d::gui::Builder;


pub struct MatchMenu {
    window: Handle<UINode>
}

impl MatchMenu {
    pub fn new(ui: &mut UserInterface) -> Self {
        let window = WindowBuilder::new(WidgetBuilder::new()
            .with_width(500.0))
            .with_title(WindowTitle::Text("Match Options"))
            .open(false)
            .with_content(GridBuilder::new(WidgetBuilder::new())
                .build(ui))
            .build(ui);

        Self {
            window
        }
    }
}