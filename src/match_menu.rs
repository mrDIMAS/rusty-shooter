use rg3d::{
    core::pool::Handle,
    gui::{
        UINode,
        UserInterface,
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
        Builder,
        text::TextBuilder,
        scroll_bar::ScrollBar,
        event::{
            UIEvent,
            UIEventKind,
        },
        button::ButtonBuilder,
        UINodeContainer,
        Control,
    },
    engine::Engine,
};
use std::sync::mpsc::Sender;
use crate::{
    message::Message,
    MatchOptions,
    menu::InterfaceTemplates,
    DeathMatch,
};

pub struct MatchMenu {
    sender: Sender<Message>,
    pub window: Handle<UINode>,
    sb_frag_limit: Handle<UINode>,
    sb_time_limit: Handle<UINode>,
    start_button: Handle<UINode>,
}

impl MatchMenu {
    pub fn new(ui: &mut UserInterface, interface_templates: &InterfaceTemplates, sender: Sender<Message>) -> Self {
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
                    .with_style(interface_templates.style.clone())
                    .on_row(0)
                    .on_column(0))
                    .with_text("Time Limit (min)")
                    .build(ui))
                .with_child({
                    sb_time_limit = interface_templates.scroll_bar.instantiate(ui);
                    ui.node_mut(sb_time_limit)
                        .downcast_mut::<ScrollBar>()
                        .unwrap()
                        .set_value(10.0)
                        .set_min_value(5.0)
                        .set_max_value(60.0)
                        .set_step(1.0)
                        .widget_mut()
                        .set_row(0)
                        .set_column(1);
                    sb_time_limit
                })
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_style(interface_templates.style.clone())
                    .on_row(1)
                    .on_column(0))
                    .with_text("Frag Limit")
                    .build(ui))
                .with_child({
                    sb_frag_limit = interface_templates.scroll_bar.instantiate(ui);
                    ui.node_mut(sb_frag_limit)
                        .downcast_mut::<ScrollBar>()
                        .unwrap()
                        .set_value(30.0)
                        .set_step(1.0)
                        .set_min_value(10.0)
                        .set_max_value(200.0)
                        .widget_mut()
                        .set_row(1)
                        .set_column(1);
                    sb_frag_limit
                })
                .with_child({
                    start_button = ButtonBuilder::new(WidgetBuilder::new()
                        .with_style(interface_templates.style.clone())
                        .on_row(2)
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

    pub fn handle_ui_event(&mut self, engine: &mut Engine, event: &UIEvent) {
        let ui = &mut engine.user_interface;

        match event.kind {
            UIEventKind::Click => {
                if event.source() == self.start_button {
                    let time_limit_minutes = ui.node(self.sb_time_limit)
                        .downcast_ref::<ScrollBar>()
                        .unwrap()
                        .value();

                    let frag_limit = ui.node(self.sb_frag_limit)
                        .downcast_ref::<ScrollBar>()
                        .unwrap()
                        .value();

                    let options = MatchOptions::DeathMatch(DeathMatch {
                        time_limit_secs: time_limit_minutes * 60.0,
                        frag_limit: frag_limit as u32,
                    });

                    self.sender
                        .send(Message::StartNewGame { options })
                        .unwrap();
                }
            }
            _ => ()
        }
    }
}