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
    physics::{
        StaticGeometry,
        StaticTriangle,
    },
    resource::model::Model,
};
use std::path::Path;

use crate::{
    player::Player,
    GameTime,
};
use rand::Rng;

use rg3d_core::{
    color::Color,
    color_gradient::{ColorGradient, GradientPoint},
    pool::*,
    visitor::{
        Visit,
        VisitResult,
        Visitor,
    },
    math::vec3::*,
};
use std::sync::Arc;

pub struct Level {
    scene: Handle<Scene>,
    player: Option<Player>,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            scene: Handle::none(),
            player: None,
        }
    }
}

impl Visit for Level {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.scene.visit("Scene", visitor)?;
        self.player.visit("Player", visitor)?;

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

    fn get_kind(&self) -> u8 {
        3
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
    pub fn new(engine: &mut Engine) -> Level {
        // Create test scene
        let mut scene = Scene::new();

        let map_model_handle = engine.get_state_mut().request_resource(Path::new("data/models/dm6.fbx"));
        if map_model_handle.is_some() {
            // Instantiate map
            let map_root_handle = Model::instantiate(map_model_handle.unwrap(), &mut scene).unwrap_or(Handle::none());

            // Create collision geometry
            let polygon_handle = scene.find_node_by_name(map_root_handle, "Polygon");
            if let Some(polygon) = scene.get_node(polygon_handle) {
                let global_transform = polygon.get_global_transform();
                let mut static_geometry = StaticGeometry::new();
                if let NodeKind::Mesh(mesh) = polygon.borrow_kind() {
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

                            if let Some(triangle) = StaticTriangle::from_points(a, b, c) {
                                static_geometry.add_triangle(triangle);
                            } else {
                                println!("degenerated triangle!");
                            }

                            i += 3;
                        }
                    }
                }
                scene.get_physics_mut().add_static_geometry(static_geometry);
            } else {
                println!("Unable to find Polygon node to build collision shape for level!");
            }
        }

        let mut ripper_handles: Vec<Handle<Node>> = Vec::new();
        let ripper_model_handle = engine.get_state_mut().request_resource(Path::new("data/models/ripper.fbx"));
        if let Some(ripper_model_resource) = ripper_model_handle {
            for _ in 0..4 {
                ripper_handles.push(Model::instantiate(Arc::clone(&ripper_model_resource), &mut scene).unwrap_or(Handle::none()));
            }
        }
        for (i, handle) in ripper_handles.iter().enumerate() {
            if let Some(node) = scene.get_node_mut(*handle) {
                node.set_local_position(Vec3::make(-0.25, -1.40, 3.0 - i as f32 * 1.40));
            }
        }

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
        if let Some(texture) = engine.get_state_mut().request_resource(Path::new("data/particles/smoke_04.tga")) {
            particle_system.set_texture(texture);
        }
        scene.add_node(Node::new(NodeKind::ParticleSystem(particle_system)));

        Level {
            player: Some(Player::new(engine.get_state_mut(), &mut scene)),
            scene: engine.get_state_mut().add_scene(scene),
        }
    }

    pub fn destroy(&mut self, engine: &mut Engine) {
        engine.get_state_mut().destroy_scene(self.scene);
    }

    pub fn get_player(&self) -> Option<&Player> {
        self.player.as_ref()
    }

    pub fn get_player_mut(&mut self) -> Option<&mut Player> {
        self.player.as_mut()
    }

    pub fn update(&mut self, engine: &mut Engine, time: &GameTime) {
        if let Some(scene) = engine.get_state_mut().get_scene_mut(self.scene) {
            if let Some(ref mut player) = self.player {
                player.update(scene, time);
            }
        }
    }
}