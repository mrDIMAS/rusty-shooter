use std::{
    path::Path,
    sync::{Mutex, Arc},
};
use crate::{
    GameTime,
    projectile::{
        Projectile,
        ProjectileKind,
        ProjectileContainer,
    },
    actor::{
        Actor,
    },
};
use rg3d_core::{
    color::Color,
    pool::Handle,
    visitor::{Visit, VisitResult, Visitor},
    math::{vec3::Vec3, ray::Ray},
};
use rg3d_physics::{RayCastOptions, Physics};
use rg3d_sound::{
    source::Source,
    buffer::BufferKind,
    context::Context,
};
use rg3d::{
    engine::resource_manager::ResourceManager,
    resource::{
        model::Model,
    },
    scene::{
        SceneInterfaceMut,
        node::{
            NodeTrait,
            Node,
        },
        Scene,
        graph::Graph,
        light::{
            LightKind,
            LightBuilder,
            PointLight,
        },
    },
};

pub enum WeaponKind {
    M4,
    Ak47,
    PlasmaRifle,
}

impl WeaponKind {
    pub fn id(&self) -> u32 {
        match self {
            WeaponKind::M4 => 0,
            WeaponKind::Ak47 => 1,
            WeaponKind::PlasmaRifle => 2
        }
    }

    pub fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(WeaponKind::M4),
            1 => Ok(WeaponKind::Ak47),
            2 => Ok(WeaponKind::PlasmaRifle),
            _ => return Err(format!("unknown weapon kind {}", id))
        }
    }
}

pub struct Weapon {
    kind: WeaponKind,
    model: Handle<Node>,
    laser_dot: Handle<Node>,
    shot_point: Handle<Node>,
    offset: Vec3,
    dest_offset: Vec3,
    last_shot_time: f64,
    shot_position: Vec3,
    owner: Handle<Actor>,
}

impl Default for Weapon {
    fn default() -> Self {
        Self {
            kind: WeaponKind::M4,
            laser_dot: Handle::NONE,
            model: Handle::NONE,
            offset: Vec3::ZERO,
            shot_point: Handle::NONE,
            dest_offset: Vec3::ZERO,
            last_shot_time: 0.0,
            shot_position: Vec3::ZERO,
            owner: Handle::NONE,
        }
    }
}

impl Visit for Weapon {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id = self.kind.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = WeaponKind::new(kind_id)?
        }

        self.model.visit("Model", visitor)?;
        self.laser_dot.visit("LaserDot", visitor)?;
        self.offset.visit("Offset", visitor)?;
        self.dest_offset.visit("DestOffset", visitor)?;
        self.last_shot_time.visit("LastShotTime", visitor)?;
        self.owner.visit("Owner", visitor)?;

        visitor.leave_region()
    }
}

impl Weapon {
    pub fn new(kind: WeaponKind, resource_manager: &mut ResourceManager, scene: &mut Scene) -> Weapon {
        let model_path = match kind {
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
        let laser_dot = graph.add_node(Node::Light(
            LightBuilder::new(LightKind::Point(PointLight::new(0.5)))
                .with_color(Color::opaque(255, 0, 0))
                .build()));

        let shot_point = graph.find_by_name(weapon_model, "Weapon:ShotPoint");

        if shot_point.is_none() {
            println!("Shot point not found!");
        }

        Weapon {
            kind,
            shot_position: Vec3::ZERO,
            laser_dot,
            model: weapon_model,
            offset: Vec3::ZERO,
            dest_offset: Vec3::ZERO,
            last_shot_time: 0.0,
            shot_point,
            owner: Handle::NONE,
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

    fn get_shot_position(&self, graph: &Graph) -> Vec3 {
        if self.shot_point.is_some() {
            graph.get(self.shot_point).get_global_position()
        } else {
            // Fallback
            graph.get(self.model).get_global_position()
        }
    }

    fn update_laser_sight(&self, graph: &mut Graph, physics: &Physics) {
        let mut laser_dot_position = Vec3::ZERO;
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
            self.offset = Vec3::new(0.0, 0.0, -0.05);
            self.last_shot_time = time.elapsed;

            self.play_shot_sound(resource_manager, sound_context);

            let (dir, pos) = {
                let graph = scene.interface().graph;
                (graph.get(self.model).get_look_vector(), self.get_shot_position(graph))
            };

            match self.kind {
                WeaponKind::M4 | WeaponKind::Ak47 => {
                    projectiles.add(Projectile::new(ProjectileKind::Bullet,
                                                    resource_manager, scene, dir, pos));
                }
                WeaponKind::PlasmaRifle => {
                    projectiles.add(Projectile::new(ProjectileKind::Plasma,
                                                    resource_manager, scene, dir, pos));
                }
            }
        }
    }
}