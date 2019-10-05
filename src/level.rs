use rg3d::{
    scene::{
        node::*,
        *,
        particle_system::{
            ParticleSystem, Emitter,
            EmitterKind, CustomEmitter, Particle,
            Emit,
        },
    },
    engine::*,
    resource::model::Model,
};
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
    pool::{Pool, Handle},
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
    projectile::Projectile,
};

pub struct Level {
    scene: Handle<Scene>,
    player: Option<Player>,
    bots: Pool<Bot>,
    projectiles: Pool<Projectile>,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            projectiles: Pool::new(),
            scene: Handle::NONE,
            player: None,
            bots: Pool::new(),
        }
    }
}

impl Visit for Level {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.scene.visit("Scene", visitor)?;
        self.player.visit("Player", visitor)?;
        self.bots.visit("Bots", visitor)?;
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
            if let Some(polygon) = graph.get(polygon_handle) {
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
        // Test particle system
        let mut particle_system = ParticleSystem::new();
        particle_system.set_acceleration(Vec3::make(0.0, -0.1, 0.0));
        let mut gradient = ColorGradient::new();
        gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
        gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(150, 150, 150, 220)));
        gradient.add_point(GradientPoint::new(0.85, Color::from_rgba(255, 255, 255, 180)));
        gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
        particle_system.set_color_over_lifetime_gradient(gradient);
        let emitter = Emitter::new(EmitterKind::Custom(Box::new(CylinderEmitter::new())));
        particle_system.add_emitter(emitter);
        if let Some(texture) = resource_manager.request_texture(Path::new("data/particles/smoke_04.tga")) {
            particle_system.set_texture(texture);
        }
        graph.add_node(Node::new(NodeKind::ParticleSystem(particle_system)));

        scene
    }

    fn create_bots(engine: &mut Engine, scene: &mut Scene) -> Pool<Bot> {
        let mut bots = Pool::new();
        bots.spawn(Bot::new(BotKind::Mutant, engine, scene).unwrap());
        bots
    }

    fn create_player(engine: &mut Engine, scene: &mut Scene) -> Player {
        let mut player = Player::new(engine, scene);
        let EngineInterfaceMut { resource_manager, .. } = engine.interface_mut();
        let ak47 = Weapon::new(WeaponKind::Ak47, resource_manager, scene);
        let m4 = Weapon::new(WeaponKind::M4, resource_manager, scene);
        let SceneInterfaceMut { graph, .. } = scene.interface_mut();
        player.add_weapon(graph, m4);
        player.add_weapon(graph, ak47);
        player
    }

    pub fn new(engine: &mut Engine) -> Level {
        let mut scene = Self::load_level(engine);
        let bots = Self::create_bots(engine, &mut scene);
        let player = Some(Self::create_player(engine, &mut scene));
        let EngineInterfaceMut { scenes, .. } = engine.interface_mut();
        let scene = scenes.add(scene);
        Level {
            projectiles: Pool::new(),
            player,
            bots,
            scene,
        }
    }

    pub fn destroy(&mut self, engine: &mut Engine) {
        let EngineInterfaceMut { scenes, .. } = engine.interface_mut();
        scenes.remove(self.scene);
    }

    pub fn get_player(&self) -> Option<&Player> {
        self.player.as_ref()
    }

    pub fn get_player_mut(&mut self) -> Option<&mut Player> {
        self.player.as_mut()
    }

    pub fn update(&mut self, engine: &mut Engine, time: &GameTime) {
        if let Some(ref mut player) = self.player {
            player.update(engine, self.scene, time, &mut self.projectiles);

            let EngineInterfaceMut { scenes, .. } = engine.interface_mut();
            if let Some(scene) = scenes.get_mut(self.scene) {
                let player_position = player.get_position(scene);

                for bot in self.bots.iter_mut() {
                    bot.update(scene, player_position, time)
                }

                // Update projectiles
                for (handle, projectile) in self.projectiles.pair_iter_mut() {
                    projectile.update(scene, time);

                    if projectile.is_dead() {
                        projectile.remove_self(scene);
                    }
                }

                self.projectiles.retain(|proj| !proj.is_dead());
            }
        }
    }
}