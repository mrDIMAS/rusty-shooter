use std::{
    path::Path,
    sync::{Arc, Mutex},
    collections::VecDeque,
};
use rg3d::{
    core::{
        color::Color,
    },
    resource::{
        texture::TextureKind,
    },
    event::{Event, WindowEvent, VirtualKeyCode, ElementState},
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
        Builder,
        UINodeContainer,
        brush::Brush,
        node::UINode,
        Control
    },
};
use crate::{
    level::LeaderBoard, GameTime, message::Message,
    MatchOptions, character::Team,
    UINodeHandle, GameEngine,
    Gui
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
    first_score: UINodeHandle,
    second_score: UINodeHandle,
}

impl Hud {
    pub fn new(engine: &mut GameEngine) -> Self {
        let leader_board = LeaderBoardUI::new(engine);

        let frame_size = engine.renderer.get_frame_size();
        let ui = &mut engine.user_interface;
        let resource_manager = &mut engine.resource_manager;

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
                    bottom: 150.0
                })
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .on_column(0)
                    .with_background(Brush::Solid(Color::opaque(249, 166, 2)))
                    .with_foreground(Brush::Solid(Color::opaque(200, 110, 0)))
                    .with_child({
                        first_score = TextBuilder::new(WidgetBuilder::new()
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_foreground(Brush::Solid(Color::BLACK)))
                            .with_text("30")
                            .build(ui);
                        first_score
                    }))
                    .with_stroke_thickness(Thickness::uniform(2.0))
                    .build(ui))
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .on_column(1)
                    .with_background(Brush::Solid(Color::opaque(127, 127, 127)))
                    .with_foreground(Brush::Solid(Color::opaque(80, 80, 80)))
                    .with_child({
                        second_score = TextBuilder::new(WidgetBuilder::new()
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .with_vertical_alignment(VerticalAlignment::Center)
                            .with_foreground(Brush::Solid(Color::BLACK)))
                            .with_text("20")
                            .build(ui);
                        second_score
                    }))
                    .with_stroke_thickness(Thickness::uniform(2.0))
                    .build(ui)))
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

    pub fn add_message<P: AsRef<str>>(&mut self, message: P) {
        self.message_queue
            .push_back(message.as_ref().to_owned())
    }

    pub fn process_input_event(&mut self, engine: &mut GameEngine, event: &Event<()>) {
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

    pub fn handle_message(&mut self, message: &Message, ui: &mut Gui, leader_board: &LeaderBoard, match_options: &MatchOptions) {
        if let Message::AddNotification { text } = message {
            self.add_message(text)
        }

        self.leader_board.handle_message(message, ui, leader_board, match_options);
    }
}

pub struct LeaderBoardUI {
    root: UINodeHandle
}

impl LeaderBoardUI {
    pub fn new(engine: &mut GameEngine) -> Self {
        let frame_size = engine.renderer.get_frame_size();

        let ui = &mut engine.user_interface;

        let root: UINodeHandle = GridBuilder::new(WidgetBuilder::new()
            .with_visibility(false)
            .with_width(frame_size.0 as f32)
            .with_height(frame_size.1 as f32))
            .add_row(Row::stretch())
            .add_row(Row::strict(600.0))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .add_column(Column::strict(500.0))
            .add_column(Column::stretch())
            .build(ui);
        Self {
            root
        }
    }

    pub fn sync_to_model(&mut self,
                         ui: &mut Gui,
                         leader_board: &LeaderBoard,
                         match_options: &MatchOptions,
    ) {
        // Rebuild entire table, this is far from ideal but it is simplest solution.
        // Shouldn't be a big problem because this method should be called once anything
        // changes in leader board.

        let row_template = Row::strict(30.0);

        let mut children = Vec::new();

        for (i, (name, score)) in leader_board.values().iter().enumerate() {
            let row = i + 1;

            children.push(TextBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0))
                .on_row(row)
                .on_column(0))
                .with_text(name)
                .build(ui));

            children.push(TextBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0))
                .on_row(row)
                .on_column(1))
                .with_text(format!("{}", score.kills))
                .build(ui));

            children.push(TextBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0))
                .on_row(row)
                .on_column(2))
                .with_text(format!("{}", score.deaths))
                .build(ui));

            let kd = if score.deaths != 0 {
                format!("{}", score.kills as f32 / score.deaths as f32)
            } else {
                "N/A".to_owned()
            };

            children.push(TextBuilder::new(WidgetBuilder::new()
                .with_margin(Thickness::uniform(3.0))
                .on_row(row)
                .on_column(3))
                .with_text(kd)
                .build(ui));
        }

        let table = GridBuilder::new(WidgetBuilder::new()
            .on_row(1)
            .on_column(1)
            .with_background(Brush::Solid(Color::BLACK))
            .with_child(TextBuilder::new(WidgetBuilder::new()
                .on_column(0)
                .on_row(0)
                .with_horizontal_alignment(HorizontalAlignment::Center))
                .with_text({
                    let time_limit_secs = match match_options {
                        MatchOptions::DeathMatch(dm) => dm.time_limit_secs,
                        MatchOptions::TeamDeathMatch(tdm) => tdm.time_limit_secs,
                        MatchOptions::CaptureTheFlag(ctf) => ctf.time_limit_secs,
                    };

                    let seconds = (time_limit_secs % 60.0) as u32;
                    let minutes = (time_limit_secs / 60.0) as u32;
                    let hours = (time_limit_secs / 3600.0) as u32;

                    match match_options {
                        MatchOptions::DeathMatch(_) => format!("Death Match - Time Limit {:02}:{:02}:{:02}", hours, minutes, seconds),
                        MatchOptions::TeamDeathMatch(_) => format!("Team Death Match - Time Limit {:02}:{:02}:{:02}", hours, minutes, seconds),
                        MatchOptions::CaptureTheFlag(_) => format!("Capture The Flag - Time Limit {:02}:{:02}:{:02}", hours, minutes, seconds),
                    }
                })
                .build(ui))
            .with_child({
                match match_options {
                    MatchOptions::DeathMatch(dm) => {
                        let text = if let Some((name, kills)) = leader_board.highest_personal_score() {
                            format!("{} leads with {} frags\nPlaying until {} frags", name, kills, dm.frag_limit)
                        } else {
                            format!("Draw\nPlaying until {} frags", dm.frag_limit)
                        };
                        TextBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(5.0))
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .on_column(0)
                            .on_row(1))
                            .with_text(text)
                            .build(ui)
                    }
                    MatchOptions::TeamDeathMatch(tdm) => {
                        let red_score = leader_board.team_score(Team::Red);
                        let blue_score = leader_board.team_score(Team::Blue);

                        TextBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(5.0))
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .on_column(0)
                            .on_row(1))
                            .with_text(format!("{} team leads\nRed {} - {} Blue\nPlaying until {} frags",
                                               if red_score > blue_score { "Red" } else { "Blue" }, red_score, blue_score, tdm.team_frag_limit))
                            .build(ui)
                    }
                    MatchOptions::CaptureTheFlag(ctf) => {
                        // TODO - implement when CTF mode implemented
                        TextBuilder::new(WidgetBuilder::new()
                            .with_margin(Thickness::uniform(5.0))
                            .with_horizontal_alignment(HorizontalAlignment::Center)
                            .on_column(0)
                            .on_row(1))
                            .with_text(format!("Red team leads\nRed 0 - 0 Blue\nPlaying until {} flags", ctf.flag_limit))
                            .build(ui)
                    }
                }
            })
            .with_child(GridBuilder::new(WidgetBuilder::new()
                .on_column(0)
                .on_row(2)
                .with_foreground(Brush::Solid(Color::opaque(120, 120, 120)))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(0)
                    .on_row(0))
                    .with_text("Name")
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(1)
                    .on_row(0))
                    .with_text("Kills")
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(2)
                    .on_row(0))
                    .with_text("Deaths")
                    .build(ui))
                .with_child(TextBuilder::new(WidgetBuilder::new()
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .on_column(3)
                    .on_row(0))
                    .with_text("K/D")
                    .build(ui))
                .with_children(&children))
                .with_border_thickness(2.0)
                .add_row(Row::strict(30.0))
                .add_rows((0..leader_board.values().len()).map(|_| row_template).collect())
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .draw_border(true)
                .build(ui)))
            .add_column(Column::auto())
            .add_row(Row::auto())
            .add_row(Row::auto())
            .add_row(Row::stretch())
            .build(ui);

        if let Some(table) = ui.node(self.root).widget().children().first() {
            let table = *table;
            ui.remove_node(table);
        }
        ui.link_nodes(table, self.root);
    }

    pub fn process_input_event(&mut self, engine: &mut GameEngine, event: &Event<()>) {
        if let Event::WindowEvent { event, .. } = event {
            match event {
                WindowEvent::Resized(new_size) => {
                    engine.user_interface
                        .node_mut(self.root)
                        .widget_mut()
                        .set_width_mut(new_size.width as f32)
                        .set_height_mut(new_size.height as f32);
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(vk) = input.virtual_keycode {
                        if vk == VirtualKeyCode::Tab {
                            let visible = match input.state {
                                ElementState::Pressed => true,
                                ElementState::Released => false,
                            };

                            engine.user_interface
                                .node_mut(self.root)
                                .widget_mut()
                                .set_visibility(visible);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn handle_message(&mut self, message: &Message, ui: &mut Gui, leader_board: &LeaderBoard, match_options: &MatchOptions) {
        match message {
            Message::AddBot { .. } => self.sync_to_model(ui, leader_board, match_options),
            Message::RemoveActor { .. } => self.sync_to_model(ui, leader_board, match_options),
            Message::SpawnBot { .. } => self.sync_to_model(ui, leader_board, match_options),
            Message::SpawnPlayer => self.sync_to_model(ui, leader_board, match_options),
            Message::RespawnActor { .. } => self.sync_to_model(ui, leader_board, match_options),
            _ => ()
        }
    }
}