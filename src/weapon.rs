use std::path::Path;

use crate::{
    GameTime,
    projectile::{Projectile, ProjectileKind, ProjectileContainer},
};
use rg3d_core::{
    color::Color,
    pool::Handle,
    visitor::{Visit, VisitResult, Visitor, VisitError},
    math::{vec3::Vec3, ray::Ray},
};
use rg3d_physics::{RayCastOptions, Physics};
use rg3d_sound::{
    source::{Source, SourceKind},
    buffer::BufferKind,
    context::Context,
};
use std::sync::{Mutex, Arc};
use rg3d::{
    engine::resource_manager::ResourceManager,
    resource::model::Model,
    scene::{
        SceneInterfaceMut,
        SceneInterface,
        node::{
            NodeKind,
            Node,
            NodeBuilder,
        },
        Scene,
        graph::Graph,
        light::LightKind,
        light::{LightBuilder, PointLight},
    },
};

pub enum WeaponKind {
    Unknown,
    M4,
    Ak47,
    PlasmaRifle,
}

pub struct Weapon {
    kind: WeaponKind,
    model: Handle<Node>,
    laser_dot: Handle<Node>,
    offset: Vec3,
    dest_offset: Vec3,
    last_shot_time: f64,
    shot_position: Vec3,
}

impl Default for Weapon {
    fn default() -> Self {
        Self {
            kind: WeaponKind::Unknown,
            laser_dot: Handle::NONE,
            model: Handle::NONE,
            offset: Vec3::new(),
            dest_offset: Vec3::new(),
            last_shot_time: 0.0,
            shot_position: Vec3::zero(),
        }
    }
}

impl Visit for Weapon {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id: u8 = if visitor.is_reading() {
            0
        } else {
            match self.kind {
                WeaponKind::Unknown => return Err(VisitError::User(String::from("unknown weapon kind on save???"))),
                WeaponKind::M4 => 0,
                WeaponKind::Ak47 => 1,
                WeaponKind::PlasmaRifle => 2
            }
        };

        kind_id.visit("KindId", visitor)?;

        if visitor.is_reading() {
            self.kind = match kind_id {
                0 => WeaponKind::M4,
                1 => WeaponKind::Ak47,
                2 => WeaponKind::PlasmaRifle,
                _ => return Err(VisitError::User(format!("unknown weapon kind {}", kind_id)))
            }
        }

        self.model.visit("Model", visitor)?;
        self.laser_dot.visit("LaserDot", visitor)?;
        self.offset.visit("Offset", visitor)?;
        self.dest_offset.visit("DestOffset", visitor)?;
        self.last_shot_time.visit("LastShotTime", visitor)?;

        visitor.leave_region()
    }
}

impl Weapon {
    pub fn new(kind: WeaponKind, resource_manager: &mut ResourceManager, scene: &mut Scene) -> Weapon {
        let model_path = match kind {
            WeaponKind::Unknown => panic!("must not be here"),
            WeaponKind::Ak47 => Path::new("data/models/ak47.fbx"),
            WeaponKind::M4 => Path::new("data/models/m4.fbx"),
            WeaponKind::PlasmaRifle => Path::new("data/models/plasma_rifle.fbx"),
        };

        let mut weapon_model = Handle::NONE;
        let model_resource_handle = resource_manager.request_model(model_path);
        if model_resource_handle.is_some() {
            weapon_model = Model::instantiate(model_resource_handle.unwrap(), scene).root;
        }

        let SceneInterfaceMut { graph, .. } = scene.interface_mut();
        let laser_dot = NodeBuilder::new(NodeKind::Light(
            LightBuilder::new(LightKind::Point(PointLight::new(0.5)))
                .with_color(Color::opaque(255, 0, 0))
                .build()))
            .build(graph);

        Weapon {
            kind,
            shot_position: Vec3::zero(),
            laser_dot,
            model: weapon_model,
            offset: Vec3::new(),
            dest_offset: Vec3::new(),
            last_shot_time: 0.0,
        }
    }

    pub fn set_visibility(&self, visibility: bool, graph: &mut Graph) {
        graph.get_mut(self.model).set_visibility(visibility);
        graph.get_mut(self.laser_dot).set_visibility(visibility);
    }

    pub fn get_model(&self) -> Handle<Node> {
        self.model
    }

    pub fn update(&mut self, scene: &mut Scene) {
        let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();

        self.offset.x += (self.dest_offset.x - self.offset.x) * 0.2;
        self.offset.y += (self.dest_offset.y - self.offset.y) * 0.2;
        self.offset.z += (self.dest_offset.z - self.offset.z) * 0.2;

        self.update_laser_sight(graph, physics);

        let node = graph.get_mut(self.model);
        node.get_local_transform_mut().set_position(self.offset);
        self.shot_position = node.get_global_position();
    }

    fn update_laser_sight(&self, graph: &mut Graph, physics: &Physics) {
        let mut laser_dot_position = Vec3::new();
        let model = graph.get(self.model);
        let begin = model.get_global_position();
        let end = begin + model.get_look_vector().scale(100.0);
        if let Some(ray) = Ray::from_two_points(&begin, &end) {
            let mut result = Vec::new();
            if physics.ray_cast(&ray, RayCastOptions::default(), &mut result) {
                let offset = result[0].normal.normalized().unwrap().scale(0.2);
                laser_dot_position = result[0].position + offset;
            }
        }

        graph.get_mut(self.laser_dot).get_local_transform_mut().set_position(laser_dot_position);
    }

    fn play_shot_sound(&self, resource_manager: &mut ResourceManager, sound_context: Arc<Mutex<Context>>) {
        let mut sound_context = sound_context.lock().unwrap();
        let shot_buffer = resource_manager.request_sound_buffer(
            Path::new("data/sounds/m4_shot.wav"), BufferKind::Normal).unwrap();
        let mut shot_sound = Source::new_spatial(shot_buffer).unwrap();
        shot_sound.set_play_once(true);
        shot_sound.play();
        shot_sound.as_spatial_mut().set_position(&self.shot_position);
        sound_context.add_source(shot_sound);
    }

    pub fn shoot(&mut self,
                 resource_manager: &mut ResourceManager,
                 scene: &mut Scene,
                 sound_context: Arc<Mutex<Context>>,
                 time: &GameTime,
                 projectiles: &mut ProjectileContainer) {
        if time.elapsed - self.last_shot_time >= 0.1 {
            self.offset = Vec3::make(0.0, 0.0, -0.05);
            self.last_shot_time = time.elapsed;

            self.play_shot_sound(resource_manager, sound_context);

            let (dir, pos) = {
                let SceneInterface { graph, .. } = scene.interface();
                let model = graph.get(self.model);
                (model.get_look_vector(), model.get_global_position())
            };

            match self.kind {
                WeaponKind::Unknown => (),
                WeaponKind::M4 => {}
                WeaponKind::Ak47 => {}
                WeaponKind::PlasmaRifle => {
                    projectiles.add(Projectile::new(
                        ProjectileKind::Plasma,
                        resource_manager,
                        scene, dir, pos));
                }
            }
        }
    }
}