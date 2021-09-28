use crate::{
    leader_board::{LeaderBoard, LeaderBoardUI},
    message::Message,
    GameTime, MatchOptions,
};
use rg3d::core::pool::Handle;
use rg3d::engine::Engine;
use rg3d::gui::{UiNode, UserInterface};
use rg3d::{
    core::color::Color,
    event::{Event, WindowEvent},
    gui::{
        border::BorderBuilder,
        brush::Brush,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{MessageDirection, TextMessage, WidgetMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        ttf::{Font, SharedFont},
        widget::WidgetBuilder,
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
    utils,
};
use std::{
    collections::VecDeque,
    path::Path,
    sync::{Arc, Mutex},
};

pub struct Hud {
    root: Handle<UiNode>,
    health: Handle<UiNode>,
    armor: Handle<UiNode>,
    ammo: Handle<UiNode>,
    time: Handle<UiNode>,
    message: Handle<UiNode>,
    message_queue: VecDeque<String>,
    message_timeout: f32,
    leader_board: LeaderBoardUI,
    match_limit: Handle<UiNode>,
    first_score: Handle<UiNode>,
    second_score: Handle<UiNode>,
    died: Handle<UiNode>,
}

impl Hud {
    pub fn new(engine: &mut Engine) -> Self {
        let leader_board = LeaderBoardUI::new(engine);

        let frame_size = engine.renderer.get_frame_size();
        let ctx = &mut engine.user_interface.build_ctx();
        let resource_manager = engine.resource_manager.clone();

        let font = rg3d::core::futures::executor::block_on(Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            35.0,
            Font::default_char_set(),
        ))
        .unwrap();
        let font = SharedFont(Arc::new(Mutex::new(font)));

        let health;
        let armor;
        let ammo;
        let message;
        let time;
        let first_score;
        let second_score;
        let match_limit;
        let died;
        let root = GridBuilder::new(
            WidgetBuilder::new()
                .with_width(frame_size.0 as f32)
                .with_height(frame_size.1 as f32)
                .with_visibility(false)
                .with_child(
                    ImageBuilder::new(
                        WidgetBuilder::new()
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_width(33.0)
                            .with_height(33.0)
                            .on_row(0)
                            .on_column(1),
                    )
                    .with_texture(utils::into_gui_texture(
                        resource_manager.request_texture("data/ui/crosshair.tga", None),
                    ))
                    .build(ctx),
                )
                .with_child({
                    time = TextBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .on_column(1)
                            .on_row(0),
                    )
                    .with_font(font.clone())
                    .with_text("00:00:00")
                    .build(ctx);
                    time
                })
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_column(0)
                            .on_row(0)
                            .with_vertical_alignment(VerticalAlignment::Bottom)
                            .with_margin(Thickness {
                                left: 50.0,
                                top: 0.0,
                                right: 0.0,
                                bottom: 150.0,
                            })
                            .with_child(
                                BorderBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(0)
                                        .with_background(Brush::Solid(Color::opaque(34, 177, 76)))
                                        .with_foreground(Brush::Solid(Color::opaque(52, 216, 101)))
                                        .with_child({
                                            match_limit = TextBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    )
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    )
                                                    .with_foreground(Brush::Solid(Color::BLACK)),
                                            )
                                            .with_text("0")
                                            .build(ctx);
                                            match_limit
                                        }),
                                )
                                .with_stroke_thickness(Thickness::uniform(2.0))
                                .build(ctx),
                            )
                            .with_child(
                                BorderBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_background(Brush::Solid(Color::opaque(249, 166, 2)))
                                        .with_foreground(Brush::Solid(Color::opaque(200, 110, 0)))
                                        .with_child({
                                            first_score = TextBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    )
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    )
                                                    .with_foreground(Brush::Solid(Color::BLACK)),
                                            )
                                            .with_text("0")
                                            .build(ctx);
                                            first_score
                                        }),
                                )
                                .with_stroke_thickness(Thickness::uniform(2.0))
                                .build(ctx),
                            )
                            .with_child(
                                BorderBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(2)
                                        .with_background(Brush::Solid(Color::opaque(127, 127, 127)))
                                        .with_foreground(Brush::Solid(Color::opaque(80, 80, 80)))
                                        .with_child({
                                            second_score = TextBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    )
                                                    .with_vertical_alignment(
                                                        VerticalAlignment::Center,
                                                    )
                                                    .with_foreground(Brush::Solid(Color::BLACK)),
                                            )
                                            .with_text("0")
                                            .build(ctx);
                                            second_score
                                        }),
                                )
                                .with_stroke_thickness(Thickness::uniform(2.0))
                                .build(ctx),
                            ),
                    )
                    .add_column(Column::strict(75.0))
                    .add_column(Column::strict(75.0))
                    .add_column(Column::strict(75.0))
                    .add_row(Row::strict(33.0))
                    .build(ctx),
                )
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::bottom(10.0))
                            .on_column(0)
                            .with_vertical_alignment(VerticalAlignment::Bottom)
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new().with_width(35.0).with_height(35.0),
                                )
                                .with_texture(utils::into_gui_texture(
                                    resource_manager
                                        .request_texture("data/ui/health_icon.png", None),
                                ))
                                .build(ctx),
                            )
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new().with_width(170.0).with_height(35.0),
                                )
                                .with_text("Health:")
                                .with_font(font.clone())
                                .build(ctx),
                            )
                            .with_child({
                                health = TextBuilder::new(
                                    WidgetBuilder::new()
                                        .with_foreground(Brush::Solid(Color::opaque(180, 14, 22)))
                                        .with_width(170.0)
                                        .with_height(35.0),
                                )
                                .with_text("100")
                                .with_font(font.clone())
                                .build(ctx);
                                health
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                )
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::bottom(10.0))
                            .on_column(1)
                            .with_vertical_alignment(VerticalAlignment::Bottom)
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new().with_width(35.0).with_height(35.0),
                                )
                                .with_texture(utils::into_gui_texture(
                                    resource_manager.request_texture("data/ui/ammo_icon.png", None),
                                ))
                                .build(ctx),
                            )
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new().with_width(170.0).with_height(35.0),
                                )
                                .with_font(font.clone())
                                .with_text("Ammo:")
                                .build(ctx),
                            )
                            .with_child({
                                ammo = TextBuilder::new(
                                    WidgetBuilder::new()
                                        .with_foreground(Brush::Solid(Color::opaque(79, 79, 255)))
                                        .with_width(170.0)
                                        .with_height(35.0),
                                )
                                .with_font(font.clone())
                                .with_text("40")
                                .build(ctx);
                                ammo
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                )
                .with_child(
                    StackPanelBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::bottom(10.0))
                            .on_column(2)
                            .with_vertical_alignment(VerticalAlignment::Bottom)
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_child(
                                ImageBuilder::new(
                                    WidgetBuilder::new().with_width(35.0).with_height(35.0),
                                )
                                .with_texture(utils::into_gui_texture(
                                    resource_manager
                                        .request_texture("data/ui/shield_icon.png", None),
                                ))
                                .build(ctx),
                            )
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new().with_width(170.0).with_height(35.0),
                                )
                                .with_font(font.clone())
                                .with_text("Armor:")
                                .build(ctx),
                            )
                            .with_child({
                                armor = TextBuilder::new(
                                    WidgetBuilder::new()
                                        .with_foreground(Brush::Solid(Color::opaque(255, 100, 26)))
                                        .with_width(170.0)
                                        .with_height(35.0),
                                )
                                .with_font(font.clone())
                                .with_text("100")
                                .build(ctx);
                                armor
                            }),
                    )
                    .with_orientation(Orientation::Horizontal)
                    .build(ctx),
                )
                .with_child({
                    message = TextBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_horizontal_alignment(HorizontalAlignment::Left)
                            .with_margin(Thickness {
                                left: 45.0,
                                top: 30.0,
                                right: 0.0,
                                bottom: 0.0,
                            })
                            .with_height(40.0)
                            .with_width(400.0),
                    )
                    .build(ctx);
                    message
                })
                .with_child({
                    died = TextBuilder::new(
                        WidgetBuilder::new()
                            .with_visibility(false)
                            .on_row(0)
                            .on_column(1)
                            .with_foreground(Brush::Solid(Color::opaque(200, 0, 0)))
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .with_font(font)
                    .with_text("You Died")
                    .build(ctx);
                    died
                }),
        )
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .build(ctx);

        Self {
            leader_board,
            root,
            health,
            armor,
            ammo,
            message,
            time,
            first_score,
            second_score,
            match_limit,
            died,
            message_timeout: 0.0,
            message_queue: Default::default(),
        }
    }

    pub fn set_health(&mut self, ui: &mut UserInterface, health: f32) {
        ui.send_message(TextMessage::text(
            self.health,
            MessageDirection::ToWidget,
            format!("{}", health),
        ));
    }

    pub fn set_armor(&mut self, ui: &mut UserInterface, armor: f32) {
        ui.send_message(TextMessage::text(
            self.armor,
            MessageDirection::ToWidget,
            format!("{}", armor),
        ));
    }

    pub fn set_ammo(&mut self, ui: &mut UserInterface, ammo: u32) {
        ui.send_message(TextMessage::text(
            self.ammo,
            MessageDirection::ToWidget,
            format!("{}", ammo),
        ));
    }

    pub fn set_visible(&mut self, ui: &mut UserInterface, visible: bool) {
        ui.send_message(WidgetMessage::visibility(
            self.root,
            MessageDirection::ToWidget,
            visible,
        ));
    }

    pub fn set_time(&mut self, ui: &mut UserInterface, time: f32) {
        let seconds = (time % 60.0) as u32;
        let minutes = (time / 60.0) as u32;
        let hours = (time / 3600.0) as u32;

        ui.send_message(TextMessage::text(
            self.time,
            MessageDirection::ToWidget,
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds),
        ));
    }

    pub fn set_is_died(&mut self, ui: &mut UserInterface, is_died: bool) {
        ui.send_message(WidgetMessage::visibility(
            self.died,
            MessageDirection::ToWidget,
            is_died,
        ));
    }

    pub fn add_message<P: AsRef<str>>(&mut self, message: P) {
        self.message_queue.push_back(message.as_ref().to_owned())
    }

    pub fn process_event(&mut self, engine: &mut Engine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::Resized(new_size) = event {
                engine.user_interface.send_message(WidgetMessage::width(
                    self.root,
                    MessageDirection::ToWidget,
                    new_size.width as f32,
                ));
                engine.user_interface.send_message(WidgetMessage::height(
                    self.root,
                    MessageDirection::ToWidget,
                    new_size.height as f32,
                ));
            }
        }

        self.leader_board.process_input_event(engine, event);
    }

    pub fn leader_board(&self) -> &LeaderBoardUI {
        &self.leader_board
    }

    pub fn update(&mut self, ui: &mut UserInterface, time: &GameTime) {
        self.message_timeout -= time.delta;

        if self.message_timeout <= 0.0 {
            if let Some(message) = self.message_queue.pop_front() {
                ui.send_message(TextMessage::text(
                    self.message,
                    MessageDirection::ToWidget,
                    message,
                ));
                self.message_timeout = 1.25;
            } else {
                ui.send_message(TextMessage::text(
                    self.message,
                    MessageDirection::ToWidget,
                    Default::default(),
                ));
            }
        }
    }

    fn update_leader_board_overview(
        &mut self,
        ui: &mut UserInterface,
        leader_board: &LeaderBoard,
        match_options: &MatchOptions,
    ) {
        // TODO: This is probably not correct way of showing leader and second place on HUD
        //  it is better to show player's score and leader/second score of some bot.
        if let Some((leader_name, leader_score)) = leader_board.highest_personal_score(None) {
            ui.send_message(TextMessage::text(
                self.first_score,
                MessageDirection::ToWidget,
                format!("{}", leader_score),
            ));

            if let Some((_, second_score)) = leader_board.highest_personal_score(Some(leader_name))
            {
                ui.send_message(TextMessage::text(
                    self.second_score,
                    MessageDirection::ToWidget,
                    format!("{}", second_score),
                ));
            }
        }

        let limit = match match_options {
            MatchOptions::DeathMatch(dm) => dm.frag_limit,
            MatchOptions::TeamDeathMatch(tdm) => tdm.team_frag_limit,
            MatchOptions::CaptureTheFlag(ctf) => ctf.flag_limit,
        };
        ui.send_message(TextMessage::text(
            self.match_limit,
            MessageDirection::ToWidget,
            format!("{}", limit),
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        ui: &mut UserInterface,
        leader_board: &LeaderBoard,
        match_options: &MatchOptions,
    ) {
        match message {
            Message::AddNotification { text } => self.add_message(text),
            Message::AddBot { .. }
            | Message::RemoveActor { .. }
            | Message::RespawnActor { .. }
            | Message::SpawnBot { .. }
            | Message::SpawnPlayer => {
                self.update_leader_board_overview(ui, leader_board, match_options)
            }
            _ => (),
        }

        self.leader_board
            .handle_message(message, ui, leader_board, match_options);
    }
}
