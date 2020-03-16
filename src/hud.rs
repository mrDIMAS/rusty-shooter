use std::{
    path::Path,
    sync::{Arc, Mutex},
    collections::VecDeque,
};
use rg3d::{
    core::color::Color,
    resource::texture::TextureKind,
    event::{
        Event,
        WindowEvent,
    },
    utils,
    gui::{
        border::BorderBuilder,
        ttf::Font,
        HorizontalAlignment,
        grid::{GridBuilder, Column, Row},
        widget::WidgetBuilder,
        text::TextBuilder,
        stack_panel::StackPanelBuilder,
        image::ImageBuilder,
        scroll_bar::Orientation,
        VerticalAlignment,
        Thickness,
        brush::Brush,
        node::UINode,
        Control,
    },
};
use crate::{
    leader_board::{
        LeaderBoard,
        LeaderBoardUI
    },
    GameTime,
    message::Message,
    MatchOptions,
    UINodeHandle,
    GameEngine,
    Gui,
};

pub struct Hud {
    root: UINodeHandle,
    health: UINodeHandle,
    armor: UINodeHandle,
    ammo: UINodeHandle,
    time: UINodeHandle,
    message: UINodeHandle,
    message_queue: VecDeque<String>,
    message_timeout: f32,
    leader_board: LeaderBoardUI,
    match_limit: UINodeHandle,
    first_score: UINodeHandle,
    second_score: UINodeHandle,
    died: UINodeHandle,
}

impl Hud {
    pub fn new(engine: &mut GameEngine) -> Self {
        let leader_board = LeaderBoardUI::new(engine);

        let frame_size = engine.renderer.get_frame_size();
        let ui = &mut engine.user_interface;
        let resource_manager = &mut engine.resource_manager.lock().unwrap();

        let font = Font::from_file(
            Path::new("data/ui/SquaresBold.ttf"),
            35.0,
            Font::default_char_set()).unwrap();
        let font = Arc::new(Mutex::new(font));

        let health;
        let armor;
        let ammo;
        let message;
        let time;
        let first_score;
        let second_score;
        let match_limit;
        let died;
        let root = GridBuilder::new(WidgetBuilder::new()
            .with_width(frame_size.0 as f32)
            .with_height(frame_size.1 as f32)
            .with_visibility(false)
            .with_child(ImageBuilder::new(WidgetBuilder::new()
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_vertical_alignment(VerticalAlignment::Center)
                .with_width(33.0)
                .with_height(33.0)
                .on_row(0)
                .on_column(1))
                .with_opt_texture(utils::into_any_arc(resource_manager.request_texture(Path::new("data/ui/crosshair.tga"), TextureKind::RGBA8)))
                .build(ui))
            .with_child({
                time = TextBuilder::new(WidgetBuilder::new()
                    .with_margin(Thickness::uniform(2.0))
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .on_column(1)
                    .on_row(0))
                    .with_font(font.clone())
                    .with_text("00:00:00")
                    .build(ui);
                time
            })
            .with_child(GridBuilder::new(WidgetBuilder::new()
                .on_column(0)
                .on_row(0)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_margin(Thickness {
                    left: 50.0,
                    top: 0.0,
                    right: 0.0,
                    bottom: 150.0,
                })
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .on_column(0)
                    .with_background(Brush::Solid(Color::opaque(34, 177, 76)))
                    .with_foreground(Brush::Solid(Color::opaque(52, 216, 101)))
                    .with_child({
                        match_limit = TextBuilder::new(WidgetBuilder::new()
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_foreground(Brush::Solid(Color::BLACK)))
                            .with_text("0")
                            .build(ui);
                        match_limit
                    }))
                    .with_stroke_thickness(Thickness::uniform(2.0))
                    .build(ui))
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .on_column(1)
                    .with_background(Brush::Solid(Color::opaque(249, 166, 2)))
                    .with_foreground(Brush::Solid(Color::opaque(200, 110, 0)))
                    .with_child({
                        first_score = TextBuilder::new(WidgetBuilder::new()
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_foreground(Brush::Solid(Color::BLACK)))
                            .with_text("0")
                            .build(ui);
                        first_score
                    }))
                    .with_stroke_thickness(Thickness::uniform(2.0))
                    .build(ui))
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .on_column(2)
                    .with_background(Brush::Solid(Color::opaque(127, 127, 127)))
                    .with_foreground(Brush::Solid(Color::opaque(80, 80, 80)))
                    .with_child({
                        second_score = TextBuilder::new(WidgetBuilder::new()
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_foreground(Brush::Solid(Color::BLACK)))
                            .with_text("0")
                            .build(ui);
                        second_score
                    }))
                    .with_stroke_thickness(Thickness::uniform(2.0))
                    .build(ui)))
                .add_column(Column::strict(75.0))
                .add_column(Column::strict(75.0))
                .add_column(Column::strict(75.0))
                .add_row(Row::strict(33.0))
                .build(ui))
            .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::bottom(10.0))
                .on_column(0)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_child(ImageBuilder::new(WidgetBuilder::new()
                    .with_width(35.0)
                    .with_height(35.0))
                    .with_opt_texture(utils::into_any_arc(resource_manager.request_texture(Path::new("data/ui/health_icon.png"), TextureKind::RGBA8)))
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_width(170.0)
                    .with_height(35.0))
                    .with_text("Health:")
                    .with_font(font.clone())
                    .build(ui))
                .with_child({
                    health = TextBuilder::new(WidgetBuilder::new()
                        .with_foreground(Brush::Solid(Color::opaque(180, 14, 22)))
                        .with_width(170.0)
                        .with_height(35.0))
                        .with_text("100")
                        .with_font(font.clone())
                        .build(ui);
                    health
                }))
                .with_orientation(Orientation::Horizontal)
                .build(ui))
            .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::bottom(10.0))
                .on_column(1)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_child(ImageBuilder::new(WidgetBuilder::new()
                    .with_width(35.0)
                    .with_height(35.0))
                    .with_opt_texture(utils::into_any_arc(resource_manager.request_texture(Path::new("data/ui/ammo_icon.png"), TextureKind::RGBA8)))
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_width(170.0)
                    .with_height(35.0))
                    .with_font(font.clone())
                    .with_text("Ammo:")
                    .build(ui)
                )
                .with_child({
                    ammo = TextBuilder::new(WidgetBuilder::new()
                        .with_foreground(Brush::Solid(Color::opaque(79, 79, 255)))
                        .with_width(170.0)
                        .with_height(35.0))
                        .with_font(font.clone())
                        .with_text("40")
                        .build(ui);
                    ammo
                }))
                .with_orientation(Orientation::Horizontal)
                .build(ui))
            .with_child(StackPanelBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::bottom(10.0))
                .on_column(2)
                .with_vertical_alignment(VerticalAlignment::Bottom)
                .with_horizontal_alignment(HorizontalAlignment::Center)
                .with_child(ImageBuilder::new(WidgetBuilder::new()
                    .with_width(35.0)
                    .with_height(35.0))
                    .with_opt_texture(utils::into_any_arc(resource_manager.request_texture(Path::new("data/ui/shield_icon.png"), TextureKind::RGBA8)))
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_width(170.0)
                    .with_height(35.0))
                    .with_font(font.clone())
                    .with_text("Armor:")
                    .build(ui))
                .with_child({
                    armor = TextBuilder::new(WidgetBuilder::new()
                        .with_foreground(Brush::Solid(Color::opaque(255, 100, 26)))
                        .with_width(170.0)
                        .with_height(35.0))
                        .with_font(font.clone())
                        .with_text("100")
                        .build(ui);
                    armor
                }))
                .with_orientation(Orientation::Horizontal)
                .build(ui))
            .with_child({
                message = TextBuilder::new(WidgetBuilder::new()
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
                    .with_width(400.0))
                    .build(ui);
                message
            })
            .with_child({
                died = TextBuilder::new(WidgetBuilder::new()
                    .with_visibility(false)
                    .on_row(0)
                    .on_column(1)
                    .with_foreground(Brush::Solid(Color::opaque(200, 0, 0)))
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .with_horizontal_alignment(HorizontalAlignment::Center))
                    .with_font(font)
                    .with_text("You Died")
                    .build(ui);
                died
            }))
            .add_column(Column::stretch())
            .add_column(Column::stretch())
            .add_column(Column::stretch())
            .add_row(Row::stretch())
            .build(ui);

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

    pub fn set_health(&mut self, ui: &mut Gui, health: f32) {
        if let UINode::Text(text) = ui.node_mut(self.health) {
            text.set_text(format!("{}", health));
        }
    }

    pub fn set_armor(&mut self, ui: &mut Gui, armor: f32) {
        if let UINode::Text(text) = ui.node_mut(self.armor) {
            text.set_text(format!("{}", armor));
        }
    }

    pub fn set_ammo(&mut self, ui: &mut Gui, ammo: u32) {
        if let UINode::Text(text) = ui.node_mut(self.ammo) {
            text.set_text(format!("{}", ammo));
        }
    }

    pub fn set_visible(&mut self, ui: &mut Gui, visible: bool) {
        ui.node_mut(self.root)
            .widget_mut()
            .set_visibility(visible);
    }

    pub fn set_time(&mut self, ui: &mut Gui, time: f32) {
        let seconds = (time % 60.0) as u32;
        let minutes = (time / 60.0) as u32;
        let hours = (time / 3600.0) as u32;

        if let UINode::Text(text) = ui.node_mut(self.time) {
            text.set_text(format!("{:02}:{:02}:{:02}", hours, minutes, seconds));
        }
    }

    pub fn set_is_died(&mut self, ui: &mut Gui, is_died: bool) {
        ui.node_mut(self.died)
            .widget_mut()
            .set_visibility(is_died);
    }

    pub fn add_message<P: AsRef<str>>(&mut self, message: P) {
        self.message_queue
            .push_back(message.as_ref().to_owned())
    }

    pub fn process_event(&mut self, engine: &mut GameEngine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::Resized(new_size) = event {
                engine.renderer
                    .set_frame_size((*new_size).into())
                    .unwrap();

                engine.user_interface
                    .node_mut(self.root)
                    .widget_mut()
                    .set_width_mut(new_size.width as f32)
                    .set_height_mut(new_size.height as f32);
            }
        }

        self.leader_board.process_input_event(engine, event);
    }

    pub fn leader_board(&self) -> &LeaderBoardUI {
        &self.leader_board
    }

    pub fn update(&mut self, ui: &mut Gui, time: &GameTime) {
        self.message_timeout -= time.delta;

        if self.message_timeout <= 0.0 {
            if let Some(message) = self.message_queue.pop_front() {
                if let UINode::Text(text) = ui.node_mut(self.message) {
                    text.set_text(message);
                }

                self.message_timeout = 1.25;
            } else {
                if let UINode::Text(text) = ui.node_mut(self.message) {
                    text.set_text("");
                }
            }
        }
    }

    fn update_leader_board_overview(&mut self, ui: &mut Gui, leader_board: &LeaderBoard, match_options: &MatchOptions) {
        // TODO: This is probably not correct way of showing leader and second place on HUD
        //  it is better to show player's score and leader/second score of some bot.
        if let Some((leader_name, leader_score)) = leader_board.highest_personal_score(None) {
            if let UINode::Text(text) = ui.node_mut(self.first_score) {
                text.set_text(format!("{}", leader_score));
            }

            if let Some((_, second_score)) = leader_board.highest_personal_score(Some(leader_name)) {
                if let UINode::Text(text) = ui.node_mut(self.second_score) {
                    text.set_text(format!("{}", second_score));
                }
            }
        }

        if let UINode::Text(text) = ui.node_mut(self.match_limit) {
            let limit = match match_options {
                MatchOptions::DeathMatch(dm) => dm.frag_limit,
                MatchOptions::TeamDeathMatch(tdm) => tdm.team_frag_limit,
                MatchOptions::CaptureTheFlag(ctf) => ctf.flag_limit,
            };

            text.set_text(format!("{}", limit));
        }
    }

    pub fn handle_message(&mut self, message: &Message, ui: &mut Gui, leader_board: &LeaderBoard, match_options: &MatchOptions) {
        match message {
            Message::AddNotification { text } => {
                self.add_message(text)
            }
            Message::AddBot { .. } => self.update_leader_board_overview(ui, leader_board, match_options),
            Message::RemoveActor { .. } => self.update_leader_board_overview(ui, leader_board, match_options),
            Message::SpawnBot { .. } => self.update_leader_board_overview(ui, leader_board, match_options),
            Message::SpawnPlayer => self.update_leader_board_overview(ui, leader_board, match_options),
            Message::RespawnActor { .. } => self.update_leader_board_overview(ui, leader_board, match_options),
            _ => ()
        }

        self.leader_board.handle_message(message, ui, leader_board, match_options);
    }
}

