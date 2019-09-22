use std::path::Path;
use rg3d::{
    resource::model::Model,
    scene::{
        node::Node,
        Scene,
    },
    engine::state::State,
};

use rg3d_core::{
    pool::Handle,
    visitor::{
        Visit,
        VisitResult,
        Visitor,
        VisitError,
    },
    math::vec3::Vec3,
};

use crate::GameTime;
use rg3d::scene::light::Light;
use rg3d_core::color::Color;
use rg3d::scene::node::NodeKind;
use rg3d_core::math::ray::Ray;
use rg3d::physics::RayCastOptions;
use rg3d::engine::Engine;
use rg3d_sound::source::{Source, SourceKind};
use rg3d_sound::buffer::BufferKind;

pub enum WeaponKind {
    Unknown,
    M4,
    Ak47,
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
            laser_dot: Handle::none(),
            model: Handle::none(),
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
            }
        };

        kind_id.visit("KindId", visitor)?;

        if visitor.is_reading() {
            self.kind = match kind_id {
                0 => WeaponKind::M4,
                1 => WeaponKind::Ak47,
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
    pub fn new(kind: WeaponKind, state: &mut State, scene: &mut Scene) -> Weapon {
        let model_path = match kind {
            WeaponKind::Unknown => panic!("must not be here"),
            WeaponKind::Ak47 => Path::new("data/models/ak47.fbx"),
            WeaponKind::M4 => Path::new("data/models/m4.fbx"),
        };

        let mut weapon_model = Handle::none();
        let model_resource_handle = state.request_model(model_path);
        if model_resource_handle.is_some() {
            weapon_model = Model::instantiate(model_resource_handle.unwrap(), scene).unwrap_or(Handle::none());
        }

        let mut light = Light::new();
        light.set_color(Color::opaque(255, 0, 0));
        light.set_radius(0.5);
        let laser_dot = scene.add_node(Node::new(NodeKind::Light(light)));

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

    pub fn set_visibility(&self, visibility: bool, scene: &mut Scene) {
        if let Some(model) = scene.get_node_mut(self.model) {
            model.set_visibility(visibility)
        }

        if let Some(laser_dot) = scene.get_node_mut(self.laser_dot) {
            laser_dot.set_visibility(visibility);
        }
    }

    pub fn get_model(&self) -> Handle<Node> {
        self.model
    }

    pub fn update(&mut self, scene: &mut Scene) {
        self.offset.x += (self.dest_offset.x - self.offset.x) * 0.2;
        self.offset.y += (self.dest_offset.y - self.offset.y) * 0.2;
        self.offset.z += (self.dest_offset.z - self.offset.z) * 0.2;

        let mut laser_dot_position = Vec3::new();
        if let Some(model) = scene.get_node(self.model) {
            let begin = model.get_global_position();
            let end = begin + model.get_look_vector().scale(100.0);
            if let Some(ray) = Ray::from_two_points(&begin, &end) {
                let mut result = Vec::new();
                if scene.get_physics().ray_cast(&ray, RayCastOptions::default(), &mut result) {
                    laser_dot_position = result[0].position + result[0].normal.normalized().unwrap().scale(0.2);
                }
            }
        }

        if let Some(laser_dot) = scene.get_node_mut(self.laser_dot) {
            laser_dot.get_local_transform_mut().set_position(laser_dot_position);
        }

        if let Some(node) = scene.get_node_mut(self.model) {
            node.get_local_transform_mut().set_position(self.offset);
            self.shot_position = node.get_global_position();
        }
    }

    pub fn shoot(&mut self, engine: &mut Engine, time: &GameTime) {
        if time.elapsed - self.last_shot_time >= 0.1 {
            self.offset = Vec3::make(0.0, 0.0, -0.05);
            self.last_shot_time = time.elapsed;

            let sound_context = engine.get_sound_context();
            let mut sound_context =sound_context.lock().unwrap();
            let shot_buffer = engine.get_state_mut().request_sound_buffer(
                Path::new("data/sounds/m4_shot.wav"), BufferKind::Normal).unwrap();
            let mut shot_sound = Source::new_spatial(shot_buffer).unwrap();
            shot_sound.set_play_once(true);
            shot_sound.play();
            if let SourceKind::Spatial(spatial) = shot_sound.get_kind_mut() {
                spatial.set_position(&self.shot_position)
            }
            sound_context.add_source(shot_sound);
        }
    }
}