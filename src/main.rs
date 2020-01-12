extern crate rg3d;
extern crate rand;

mod actor;
mod level;
mod player;
mod weapon;
mod bot;
mod projectile;
mod menu;
mod effects;
mod character;
mod hud;
mod jump_pad;
mod item;

use std::{
    fs::File,
    path::Path,
    time::Instant,
    io::Write,
    time,
    thread,
    time::Duration,
    sync::{Arc, Mutex},
};
use rg3d::{
    core::{
        pool::Handle,
        visitor::{
            Visitor,
            VisitResult,
            Visit,
        },
        color::Color,
        math::{
            vec3::Vec3,
            mat4::Mat4,
            quat::Quat,
        },
    },
    sound::{
        source::{
            Status,
            SoundSource,
            spatial::SpatialSourceBuilder,
            generic::GenericSourceBuilder,
        },
        context::Context,
    },
    gui::{
        widget::WidgetBuilder,
        UINode,
        text::TextBuilder,
        event::{UIEvent, UIEventKind},
    },
    scene::{
        particle_system::CustomEmitterFactory,
        Scene,
    },
    event::{WindowEvent, ElementState, VirtualKeyCode, Event},
    event_loop::{EventLoop, ControlFlow},
    engine::{
        resource_manager::ResourceManager,
        Engine,
        EngineInterfaceMut,
        EngineInterface,
    },
};
use crate::{
    jump_pad::JumpPadContainer,
    character::AsCharacter,
    projectile::ProjectileContainer,
    level::{Level, CylinderEmitter},
    menu::Menu,
    hud::Hud,
    weapon::WeaponContainer,
    item::ItemContainer,
};
use rg3d::gui::text::Text;
use rg3d::gui::{Builder, UINodeContainer};

pub struct Game {
    menu: Menu,
    hud: Hud,
    engine: Engine,
    level: Option<Level>,
    debug_text: Handle<UINode>,
    debug_string: String,
    running: bool,
    last_tick_time: time::Instant,
    music: Handle<SoundSource>,
}

pub trait HandleFromSelf<T> {
    fn self_handle(&self) -> Handle<T>;
}

#[derive(Copy, Clone)]
pub struct GameTime {
    elapsed: f64,
    delta: f32,
}

pub struct LevelUpdateContext<'a> {
    time: GameTime,
    scene: &'a mut Scene,
    sound_context: Arc<Mutex<Context>>,
    projectiles: &'a mut ProjectileContainer,
    resource_manager: &'a mut ResourceManager,
    items: &'a mut ItemContainer,
    weapons: &'a mut WeaponContainer,
    jump_pads: &'a mut JumpPadContainer,
}

pub enum CollisionGroups {
    Generic = 1 << 0,
    Projectile = 1 << 1,
    Actor = 1 << 2,
    All = std::isize::MAX,
}

impl Game {
    pub fn run() {
        let events_loop = EventLoop::<()>::new();

        let primary_monitor = events_loop.primary_monitor();
        let mut monitor_dimensions = primary_monitor.size();
        monitor_dimensions.height *= 0.7;
        monitor_dimensions.width *= 0.7;
        let client_size = monitor_dimensions.to_logical(primary_monitor.hidpi_factor());

        let window_builder = rg3d::window::WindowBuilder::new()
            .with_title("Rusty Shooter")
            .with_inner_size(client_size)
            .with_resizable(true);

        let mut engine = Engine::new(window_builder, &events_loop).unwrap();
        let mut hrtf_sphere = rg3d::sound::hrtf::HrtfSphere::new("data/sounds/IRC_1040_C.bin").unwrap();
        engine.interface_mut().sound_context
            .lock()
            .unwrap()
            .set_renderer(rg3d::sound::renderer::Renderer::HrtfRenderer(rg3d::sound::hrtf::HrtfRenderer::new(hrtf_sphere)));

        let frame_size = engine.interface().renderer.get_frame_size();

        if let Ok(mut factory) = CustomEmitterFactory::get() {
            factory.set_callback(Box::new(|kind| {
                match kind {
                    0 => Ok(Box::new(CylinderEmitter::new())),
                    _ => Err(String::from("invalid custom emitter kind"))
                }
            }))
        }

        let EngineInterfaceMut { sound_context, ui, resource_manager, renderer, .. } = engine.interface_mut();
        renderer.set_ambient_color(Color::opaque(60, 60, 60));

        let buffer = resource_manager.request_sound_buffer("data/sounds/Antonio_Bizarro_Berzerker.wav", true).unwrap();
        let music = sound_context.lock()
            .unwrap()
            .add_source(GenericSourceBuilder::new(buffer)
                .with_looping(true)
                .with_status(Status::Playing)
                .with_gain(0.25)
                .build_source()
                .unwrap());

        let mut reverb = rg3d::sound::effects::reverb::Reverb::new();
        reverb.set_decay_time(Duration::from_secs_f32(5.0));
        sound_context.lock().unwrap().add_effect(rg3d::sound::effects::Effect::Reverb(reverb));

        let mut game = Game {
            hud: Hud::new(ui, resource_manager, frame_size),
            running: true,
            menu: Menu::new(&mut engine),
            debug_text: Handle::NONE,
            engine,
            level: None,
            debug_string: String::new(),
            last_tick_time: time::Instant::now(),
            music,
        };

        game.create_debug_ui();

        let fixed_fps = 60.0;
        let fixed_timestep = 1.0 / fixed_fps;
        let clock = Instant::now();
        let mut game_time = GameTime {
            elapsed: 0.0,
            delta: fixed_timestep,
        };

        events_loop.run(move |event, _, control_flow| {
            match event {
                Event::EventsCleared => {
                    let mut dt = clock.elapsed().as_secs_f64() - game_time.elapsed;
                    while dt >= fixed_timestep as f64 {
                        dt -= fixed_timestep as f64;
                        game_time.elapsed += fixed_timestep as f64;

                        while let Some(mut ui_event) = game.engine.get_ui_mut().poll_ui_event() {
                            game.menu.process_ui_event(&mut game.engine, &mut ui_event);
                            game.process_ui_event(&mut ui_event);
                        }

                        game.update(game_time);
                    }
                    if !game.running {
                        *control_flow = ControlFlow::Exit;
                    }
                    game.engine.get_window().request_redraw();
                }
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::RedrawRequested => {
                            game.update_statistics(game_time.elapsed);
                            // Render at max speed
                            game.engine.render().unwrap();
                            // Make sure to cap update rate to 60 FPS.
                            game.limit_fps(fixed_fps as f64);
                        }
                        WindowEvent::CloseRequested => {
                            game.destroy_level();
                            *control_flow = ControlFlow::Exit
                        }
                        _ => {
                            game.process_input_event(event);
                        }
                    }
                }
                _ => *control_flow = ControlFlow::Poll,
            }
        });
    }

    pub fn create_debug_ui(&mut self) {
        let EngineInterfaceMut { ui, .. } = self.engine.interface_mut();

        self.debug_text = TextBuilder::new(WidgetBuilder::new()
            .with_width(400.0)
            .with_height(200.0))
            .build(ui);
    }

    pub fn save_game(&mut self) -> VisitResult {
        let mut visitor = Visitor::new();

        // Visit engine state first.
        self.engine.visit("Engine", &mut visitor)?;

        self.level.visit("Level", &mut visitor)?;

        // Debug output
        if let Ok(mut file) = File::create(Path::new("save.txt")) {
            file.write_all(visitor.save_text().as_bytes()).unwrap();
        }

        visitor.save_binary(Path::new("save.bin"))
    }

    pub fn load_game(&mut self) {
        match Visitor::load_binary(Path::new("save.bin")) {
            Ok(mut visitor) => {
                // Clean up.
                self.destroy_level();

                // Load engine state first
                match self.engine.visit("Engine", &mut visitor) {
                    Ok(_) => {
                        println!("Engine state successfully loaded!");

                        // Then load game state.
                        match self.level.visit("Level", &mut visitor) {
                            Ok(_) => {
                                println!("Game state successfully loaded!");

                                // Hide menu only of we successfully loaded a save.
                                self.set_menu_visible(false)
                            }
                            Err(e) => println!("Failed to load game state! Reason: {}", e)
                        }
                    }
                    Err(e) => println!("Failed to load engine state! Reason: {}", e)
                }
            }
            Err(e) => {
                println!("failed to load a save, reason: {}", e);
            }
        }
    }

    fn destroy_level(&mut self) {
        if let Some(ref mut level) = self.level.take() {
            level.destroy(&mut self.engine);
        }
    }

    pub fn start_new_game(&mut self) {
        self.destroy_level();
        self.level = Some(Level::new(&mut self.engine));
        self.set_menu_visible(false);
    }

    pub fn process_ui_event(&mut self, event: &mut UIEvent) {
        match event.kind {
            UIEventKind::Click => {
                if event.source() == self.menu.btn_new_game {
                    self.start_new_game();
                    event.handled = true;
                } else if event.source() == self.menu.btn_save_game {
                    match self.save_game() {
                        Ok(_) => println!("successfully saved"),
                        Err(e) => println!("failed to make a save, reason: {}", e),
                    }
                    event.handled = true;
                } else if event.source() == self.menu.btn_load_game {
                    self.load_game();
                    event.handled = true;
                } else if event.source() == self.menu.btn_quit_game {
                    self.destroy_level();
                    self.running = false;
                    event.handled = true;
                }
            }
            UIEventKind::NumericValueChanged { new_value, .. } => {
                if event.source() == self.menu.sb_music_volume {
                    self.engine
                        .interface_mut()
                        .sound_context
                        .lock()
                        .unwrap()
                        .source_mut(self.music)
                        .generic_mut()
                        .set_gain(new_value);
                }
            }

            _ => ()
        }
    }

    pub fn set_menu_visible(&mut self, visible: bool) {
        let EngineInterfaceMut { ui, .. } = self.engine.interface_mut();
        self.menu.set_visible(ui, visible);
        self.hud.set_visible(ui, !visible);
    }

    pub fn is_menu_visible(&self) -> bool {
        let EngineInterface { ui, .. } = self.engine.interface();
        self.menu.is_visible(ui)
    }

    pub fn update(&mut self, time: GameTime) {
        self.engine.update(time.delta);

        if let Some(ref mut level) = self.level {
            level.update(&mut self.engine, time);

            let player = level.get_player();
            if player.is_some() {
                // Sync hud with player state.
                let EngineInterfaceMut { ui, .. } = self.engine.interface_mut();
                let player = level.get_actors().get(player);
                self.hud.set_health(ui, player.character().get_health());
                self.hud.set_armor(ui, player.character().get_armor());
                let current_weapon = player.character().get_current_weapon();
                if current_weapon.is_some() {
                    let current_weapon = level.get_weapons().get(current_weapon);
                    self.hud.set_ammo(ui, current_weapon.get_ammo());
                }
            }
        }
    }

    pub fn update_statistics(&mut self, elapsed: f64) {
        let EngineInterfaceMut { ui, renderer, sound_context, .. } = self.engine.interface_mut();

        self.debug_string.clear();
        use std::fmt::Write;
        let statistics = renderer.get_statistics();
        write!(self.debug_string,
               "Pure frame time: {:.2} ms\n\
               Capped frame time: {:.2} ms\n\
               FPS: {}\n\
               Triangles: {}\n\
               Draw calls: {}\n\
               Up time: {:.2} s\n\
               Sound render time: {:?}",
               statistics.pure_frame_time * 1000.0,
               statistics.capped_frame_time * 1000.0,
               statistics.frames_per_second,
               statistics.geometry.triangles_rendered,
               statistics.geometry.draw_calls,
               elapsed,
               sound_context.lock().unwrap().full_render_duration()
        ).unwrap();

        ui.node_mut(self.debug_text)
            .downcast_mut::<Text>()
            .unwrap()
            .set_text(self.debug_string.as_str());
    }

    pub fn limit_fps(&mut self, value: f64) {
        let current_time = time::Instant::now();
        let render_call_duration = current_time.duration_since(self.last_tick_time).as_secs_f64();
        self.last_tick_time = current_time;
        let desired_frame_time = 1.0 / value;
        if render_call_duration < desired_frame_time {
            thread::sleep(Duration::from_secs_f64(desired_frame_time - render_call_duration));
        }
    }

    fn process_dispatched_event(&mut self, event: &WindowEvent) {
        let EngineInterfaceMut { ui, .. } = self.engine.interface_mut();

        // Some events can be consumed so they won't be dispatched further,
        // this allows to catch events by UI for example and don't send them
        // to player controller so when you click on some button in UI you
        // won't shoot from your current weapon in game.
        let event_processed = ui.process_input_event(event);

        if !event_processed {
            if let Some(ref mut level) = self.level {
                level.process_input_event(event);
            }
        }
    }

    pub fn process_input_event(&mut self, event: WindowEvent) {
        self.process_dispatched_event(&event);

        // Some events processed in any case.
        match event {
            WindowEvent::CloseRequested => self.running = false,
            WindowEvent::KeyboardInput { input, .. } => {
                if let ElementState::Pressed = input.state {
                    if let Some(key) = input.virtual_keycode {
                        if key == VirtualKeyCode::Escape {
                            self.set_menu_visible(!self.is_menu_visible());
                        }
                    }
                }
            }
            _ => ()
        }

        self.menu.process_input_event(&mut self.engine, &event);
        self.hud.process_input_event(&mut self.engine, &event);
    }
}

fn main() {
    Game::run();
}