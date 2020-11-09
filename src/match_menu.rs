use crate::{
    gui::{create_scroll_bar, ScrollBarData},
    message::Message,
    DeathMatch, GameEngine, Gui, GuiMessage, MatchOptions, UINodeHandle,
};
use rg3d::{
    engine::resource_manager::ResourceManager,
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{ButtonMessage, UiMessageData},
        node::UINode,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
};
use std::sync::mpsc::Sender;

pub struct MatchMenu {
    sender: Sender<Message>,
    pub window: UINodeHandle,
    sb_frag_limit: UINodeHandle,
    sb_time_limit: UINodeHandle,
    start_button: UINodeHandle,
}

impl MatchMenu {
    pub fn new(ui: &mut Gui, resource_manager: ResourceManager, sender: Sender<Message>) -> Self {
        let common_row = Row::strict(36.0);

        let ctx = &mut ui.build_ctx();
        let sb_frag_limit;
        let sb_time_limit;
        let start_button;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0))
            .with_title(WindowTitle::text("Match Options"))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                                .with_text("Match Type")
                                .build(ctx),
                        )
                        .with_child(
                            DropdownListBuilder::new(WidgetBuilder::new().on_column(1).on_row(0))
                                .with_items({
                                    let mut items = Vec::new();
                                    for mode in
                                        ["Deathmatch", "Team Deathmatch", "Capture The Flag"].iter()
                                    {
                                        let item = DecoratorBuilder::new(BorderBuilder::new(
                                            WidgetBuilder::new().with_height(30.0).with_child(
                                                TextBuilder::new(
                                                    WidgetBuilder::new()
                                                        .with_horizontal_alignment(
                                                            HorizontalAlignment::Center,
                                                        )
                                                        .with_vertical_alignment(
                                                            VerticalAlignment::Center,
                                                        ),
                                                )
                                                .with_text(mode)
                                                .build(ctx),
                                            ),
                                        ))
                                        .build(ctx);
                                        items.push(item);
                                    }
                                    items
                                })
                                .build(ctx),
                        )
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(1).on_column(0))
                                .with_text("Time Limit (min)")
                                .build(ctx),
                        )
                        .with_child({
                            sb_time_limit = create_scroll_bar(
                                ctx,
                                resource_manager.clone(),
                                ScrollBarData {
                                    min: 5.0,
                                    max: 60.0,
                                    value: 10.0,
                                    step: 1.0,
                                    row: 1,
                                    column: 1,
                                    margin: Thickness::uniform(2.0),
                                    show_value: true,
                                    orientation: Orientation::Horizontal,
                                },
                            );
                            sb_time_limit
                        })
                        .with_child(
                            TextBuilder::new(WidgetBuilder::new().on_row(2).on_column(0))
                                .with_text("Frag Limit")
                                .build(ctx),
                        )
                        .with_child({
                            sb_frag_limit = create_scroll_bar(
                                ctx,
                                resource_manager.clone(),
                                ScrollBarData {
                                    min: 10.0,
                                    max: 200.0,
                                    value: 30.0,
                                    step: 1.0,
                                    row: 2,
                                    column: 1,
                                    margin: Thickness::uniform(2.0),
                                    show_value: true,
                                    orientation: Orientation::Horizontal,
                                },
                            );
                            sb_frag_limit
                        })
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(3)
                                    .on_column(0)
                                    .with_margin(Thickness::uniform(2.0)),
                            )
                            .with_text("Player Name")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .build(ctx),
                        )
                        .with_child(
                            TextBoxBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(3)
                                    .on_column(1)
                                    .with_margin(Thickness::uniform(2.0)),
                            )
                            .with_text("Unnamed Player".to_owned())
                            .build(ctx),
                        )
                        .with_child({
                            start_button =
                                ButtonBuilder::new(WidgetBuilder::new().on_row(4).on_column(1))
                                    .with_text("Start")
                                    .build(ctx);
                            start_button
                        }),
                )
                .add_column(Column::strict(200.0))
                .add_column(Column::stretch())
                .add_row(common_row)
                .add_row(common_row)
                .add_row(common_row)
                .add_row(common_row)
                .add_row(common_row)
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);
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

        if let UiMessageData::Button(msg) = message.data() {
            if let ButtonMessage::Click = msg {
                if message.destination() == self.start_button {
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

                    self.sender.send(Message::StartNewGame { options }).unwrap();
                }
            }
        }
    }
}
