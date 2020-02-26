use rg3d::{
    gui::{
        HorizontalAlignment,
        VerticalAlignment,
        combobox::ComboBoxBuilder,
        Thickness,
        window::{
            WindowBuilder,
            WindowTitle,
        },
        widget::WidgetBuilder,
        grid::{
            GridBuilder,
            Row,
            Column,
        },
        scroll_bar::Orientation,
        text::TextBuilder,
        message::{
            UiMessageData,
            ButtonMessage,
        },
        button::ButtonBuilder,
        Control,
        node::UINode,
        text_box::TextBoxBuilder,
        decorator::DecoratorBuilder,
        border::BorderBuilder,
    },
    engine::resource_manager::ResourceManager,
};
use std::sync::mpsc::Sender;
use crate::{
    message::Message,
    MatchOptions,
    DeathMatch,
    UINodeHandle,
    GameEngine,
    Gui,
    GuiMessage,
    gui::create_scroll_bar,
};

pub struct MatchMenu {
    sender: Sender<Message>,
    pub window: UINodeHandle,
    sb_frag_limit: UINodeHandle,
    sb_time_limit: UINodeHandle,
    start_button: UINodeHandle,
}

impl MatchMenu {
    pub fn new(ui: &mut Gui, resource_manager: &mut ResourceManager, sender: Sender<Message>) -> Self {
        let common_row = Row::strict(36.0);

        let sb_frag_limit;
        let sb_time_limit;
        let start_button;
        let window = WindowBuilder::new(WidgetBuilder::new()
            .with_width(500.0))
            .with_title(WindowTitle::Text("Match Options"))
            .open(false)
            .with_content(GridBuilder::new(WidgetBuilder::new()
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(0)
                    .on_column(0))
                    .with_text("Match Type")
                    .build(ui))
                .with_child(ComboBoxBuilder::new(WidgetBuilder::new()
                    .on_column(1)
                    .on_row(0))
                    .with_items({
                        let mut items = Vec::new();
                        for mode in ["Deathmatch", "Team Deathmatch", "Capture The Flag"].iter() {
                            let item = DecoratorBuilder::new(
                                BorderBuilder::new(
                                    WidgetBuilder::new()
                                        .with_height(30.0)
                                        .with_child(TextBuilder::new(WidgetBuilder::new()
                                            .with_horizontal_alignment(HorizontalAlignment::Center)
                                            .with_vertical_alignment(VerticalAlignment::Center))
                                            .with_text(mode)
                                            .build(ui))))
                                .build(ui);
                            items.push(item);
                        }
                        items
                    })
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(1)
                    .on_column(0))
                    .with_text("Time Limit (min)")
                    .build(ui))
                .with_child({
                    sb_time_limit = create_scroll_bar(ui, resource_manager, Orientation::Horizontal, true);
                    if let UINode::ScrollBar(scroll_bar) = ui.node_mut(sb_time_limit) {
                        scroll_bar.set_value(10.0)
                            .set_min_value(5.0)
                            .set_max_value(60.0)
                            .set_step(1.0)
                            .widget_mut()
                            .set_row(1)
                            .set_column(1)
                            .set_margin(Thickness::uniform(2.0));
                    }
                    sb_time_limit
                })
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(2)
                    .on_column(0))
                    .with_text("Frag Limit")
                    .build(ui))
                .with_child({
                    sb_frag_limit = create_scroll_bar(ui, resource_manager, Orientation::Horizontal, true);
                    if let UINode::ScrollBar(scroll_bar) = ui.node_mut(sb_frag_limit) {
                        scroll_bar.set_value(30.0)
                            .set_step(1.0)
                            .set_min_value(10.0)
                            .set_max_value(200.0)
                            .widget_mut()
                            .set_row(2)
                            .set_column(1)
                            .set_margin(Thickness::uniform(2.0));
                    }
                    sb_frag_limit
                })
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .on_row(3)
                    .on_column(0)
                    .with_margin(Thickness::uniform(2.0)))
                    .with_text("Player Name")
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ui))
                .with_child(TextBoxBuilder::new(WidgetBuilder::new()
                    .on_row(3)
                    .on_column(1)
                    .with_margin(Thickness::uniform(2.0)))
                    .with_text("Unnamed Player".to_owned())
                    .build(ui))
                .with_child({
                    start_button = ButtonBuilder::new(WidgetBuilder::new()
                        .on_row(4)
                        .on_column(1))
                        .with_text("Start")
                        .build(ui);
                    start_button
                }))
                .add_column(Column::strict(200.0))
                .add_column(Column::stretch())
                .add_row(common_row)
                .add_row(common_row)
                .add_row(common_row)
                .add_row(common_row)
                .add_row(common_row)
                .add_row(Row::stretch())
                .build(ui))
            .build(ui);
        Self {
            sender,
            window,
            sb_frag_limit,
            sb_time_limit,
            start_button,
        }
    }

    pub fn handle_ui_event(&mut self, engine: &mut GameEngine, message: &GuiMessage) {
        let ui = &mut engine.user_interface;

        if let UiMessageData::Button(msg) = &message.data {
            if let ButtonMessage::Click = msg {
                if message.source() == self.start_button {
                    let time_limit_minutes =
                        if let UINode::ScrollBar(scroll_bar) = ui.node(self.sb_time_limit) {
                            scroll_bar.value()
                        } else {
                            0.0
                        };

                    let frag_limit =
                        if let UINode::ScrollBar(scroll_bar) = ui.node(self.sb_frag_limit) {
                            scroll_bar.value()
                        } else {
                            0.0
                        };

                    let options = MatchOptions::DeathMatch(DeathMatch {
                        time_limit_secs: time_limit_minutes * 60.0,
                        frag_limit: frag_limit as u32,
                    });

                    self.sender
                        .send(Message::StartNewGame { options })
                        .unwrap();
                }
            }
        }
    }
}