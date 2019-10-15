use rg3d::{scene::{
    node::*,
    *,
    particle_system::{
        ParticleSystem, Emitter,
        EmitterKind, CustomEmitter, Particle,
        Emit,
    },
    particle_system::{ParticleSystemBuilder, EmitterBuilder},
}, engine::*, resource::{
    model::Model,
    texture::TextureKind,
}, WindowEvent};
use std::{
    path::Path
};
use rg3d_physics::static_geometry::{
    StaticGeometry,
    StaticTriangle,
};
use rand::Rng;
use rg3d_core::{
    color::Color,
    color_gradient::{ColorGradient, GradientPoint},
    pool::{Handle},
    visitor::{
        Visit,
        VisitResult,
        Visitor,
    },
    math::vec3::*,
};
use crate::{
    weapon::{Weapon, WeaponKind},
    player::Player,
    GameTime,
    bot::{
        Bot,
        BotKind,
    },
    projectile::ProjectileContainer,
};
use crate::actor::{ActorContainer, Actor};
use rg3d::engine::resource_manager::ResourceManager;
use rg3d_sound::context::Context;
use std::sync::{Arc, Mutex};
use crate::actor::ActorTrait;

pub struct Level {
    scene: Handle<Scene>,
    player: Handle<Actor>,
    projectiles: ProjectileContainer,
    actors: ActorContainer,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            projectiles: ProjectileContainer::new(),
            actors: ActorContainer::new(),
            scene: Handle::NONE,
            player: Handle::NONE,
        }
    }
}

impl Visit for Level {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.scene.visit("Scene", visitor)?;
        self.player.visit("Player", visitor)?;
        self.actors.visit("Actors", visitor)?;
        self.projectiles.visit("Projectiles", visitor)?;

        visitor.leave_region()
    }
}

#[derive(Copy, Clone)]
pub struct CylinderEmitter {
    height: f32,
    radius: f32,
}

impl CylinderEmitter {
    pub fn new() -> Self {
        Self {
            height: 1.0,
            radius: 0.5,
        }
    }
}

impl CustomEmitter for CylinderEmitter {
    fn box_clone(&self) -> Box<dyn CustomEmitter> {
        Box::new(self.clone())
    }

    fn get_kind(&self) -> i32 {
        0
    }
}

impl Visit for CylinderEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;
        self.height.visit("Height", visitor)?;

        visitor.leave_region()
    }
}

impl Emit for CylinderEmitter {
    fn emit(&self, _emitter: &Emitter, _particle_system: &ParticleSystem, particle: &mut Particle) {
        // Disk point picking extended in 3D - http://mathworld.wolfram.com/DiskPointPicking.html
        let s: f32 = rand::thread_rng().gen_range(0.0, 1.0);
        let theta = rand::thread_rng().gen_range(0.0, 2.0 * std::f32::consts::PI);
        let z = rand::thread_rng().gen_range(0.0, self.height);
        let r = s.sqrt() * self.radius;
        let x = r * theta.cos();
        let y = r * theta.sin();
        particle.position = Vec3::make(x, y, z);
    }
}

impl Level {
    fn load_level(engine: &mut Engine) -> Scene {
        let mut scene = Scene::new();

        let EngineInterfaceMut { resource_manager, .. } = engine.interface_mut();
        let map_model_handle = resource_manager.request_model(Path::new("data/models/dm6.fbx"));
        if map_model_handle.is_some() {
            // Instantiate map
            let map_root_handle = Model::instantiate(map_model_handle.unwrap(), &mut scene).root;
            let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();
            // Create collision geometry
            let polygon_handle = graph.find_by_name(map_root_handle, "Polygon");
            if polygon_handle.is_some() {
                let polygon = graph.get(polygon_handle);
                let global_transform = polygon.get_global_transform();
                let mut static_geometry = StaticGeometry::new();
                if let NodeKind::Mesh(mesh) = polygon.get_kind() {
                    for surface in mesh.get_surfaces() {
                        let data_rc = surface.get_data();
                        let shared_data = data_rc.lock().unwrap();

                        let vertices = shared_data.get_vertices();
                        let indices = shared_data.get_indices();

                        let last = indices.len() - indices.len() % 3;
                        let mut i: usize = 0;
                        while i < last {
                            let a = global_transform.transform_vector(vertices[indices[i] as usize].position);
                            let b = global_transform.transform_vector(vertices[indices[i + 1] as usize].position);
                            let c = global_transform.transform_vector(vertices[indices[i + 2] as usize].position);

                            if let Some(triangle) = StaticTriangle::from_points(&a, &b, &c) {
                                static_geometry.add_triangle(triangle);
                            } else {
                                println!("degenerated triangle!");
                            }

                            i += 3;
                        }
                    }
                }
                physics.add_static_geometry(static_geometry);
            } else {
                println!("Unable to find Polygon node to build collision shape for level!");
            }
        }

        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

        NodeBuilder::new(NodeKind::ParticleSystem(
            ParticleSystemBuilder::new()
                .with_acceleration(Vec3::make(0.0, -0.1, 0.0))
                .with_color_over_lifetime_gradient({
                    let mut gradient = ColorGradient::new();
                    gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
                    gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(150, 150, 150, 220)));
                    gradient.add_point(GradientPoint::new(0.85, Color::from_rgba(255, 255, 255, 180)));
                    gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
                    gradient
                })
                .with_emitters(vec![
                    EmitterBuilder::new(EmitterKind::Custom(Box::new(CylinderEmitter::new()))).build()
                ])
                .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/smoke_04.tga"), TextureKind::R8))
                .build()))
            .build(graph);

        scene
    }

    fn create_player(sound_context: Arc<Mutex<Context>>, resource_manager: &mut ResourceManager, scene: &mut Scene) -> Player {
        let mut player = Player::new(sound_context, resource_manager, scene);
        let plasma_rifle = Weapon::new(WeaponKind::PlasmaRifle, resource_manager, scene);
        let ak47 = Weapon::new(WeaponKind::Ak47, resource_manager, scene);
        let m4 = Weapon::new(WeaponKind::M4, resource_manager, scene);
        let SceneInterfaceMut { graph, .. } = scene.interface_mut();
        player.add_weapon(graph, m4);
        player.add_weapon(graph, ak47);
        player.add_weapon(graph, plasma_rifle);
        player
    }

    pub fn new(engine: &mut Engine) -> Level {
        let mut scene = Self::load_level(engine);
        let EngineInterfaceMut { scenes, sound_context, resource_manager, .. } = engine.interface_mut();
        let mut actors = ActorContainer::new();
        let player = actors.add(Actor::Player(Self::create_player(sound_context, resource_manager, &mut scene)));
        actors.add(Actor::Bot(Bot::new(BotKind::Mutant, resource_manager, &mut scene, Vec3::make(0.0, 0.0, -1.0)).unwrap()));
        actors.add(Actor::Bot(Bot::new(BotKind::Mutant, resource_manager, &mut scene, Vec3::make(1.0, 0.0, 0.0)).unwrap()));
        let scene = scenes.add(scene);
        Level {
            projectiles: ProjectileContainer::new(),
            actors,
            player,
            scene,
        }
    }

    pub fn destroy(&mut self, engine: &mut Engine) {
        let EngineInterfaceMut { scenes, .. } = engine.interface_mut();
        scenes.remove(self.scene);
    }

    pub fn get_player(&self) -> Handle<Actor> {
        self.player
    }

    pub fn process_input_event(&mut self, event: &WindowEvent) -> bool {
        if let Actor::Player(player) = self.actors.get_mut(self.player) {
            player.process_input_event(event)
        } else {
            false
        }
    }

    pub fn update(&mut self, engine: &mut Engine, time: &GameTime) {
        let EngineInterfaceMut { scenes, sound_context, resource_manager, .. } = engine.interface_mut();
        let scene = scenes.get_mut(self.scene);

        let player_position = self.actors.get(self.player).get_position(scene.interface().physics);

        for actor in self.actors.iter_mut() {
            if let Actor::Bot(bot) = actor {
                bot.set_target(player_position);
            }
        }

        self.actors.update(sound_context, resource_manager, scene, time, &mut self.projectiles);

        self.projectiles.update(scene, resource_manager, &mut self.actors, time);
    }
}