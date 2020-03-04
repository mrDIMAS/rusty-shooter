use std::{
    path::Path,
    sync::mpsc::Sender,
    path::PathBuf,
};
use rg3d::{
    physics::{RayCastOptions, HitKind, Physics},
    engine::resource_manager::ResourceManager,
    scene::{
        node::Node,
        Scene,
        graph::Graph,
        light::{
            LightKind,
            LightBuilder,
            PointLight,
        },
        base::{BaseBuilder, AsBase},
    },
    core::{
        pool::{
            Pool,
            PoolIteratorMut,
            Handle,
        },
        color::Color,
        visitor::{
            Visit,
            VisitResult,
            Visitor,
        },
        math::{vec3::Vec3, ray::Ray},
    },
};
use crate::{
    actor::ActorContainer,
    projectile::ProjectileKind,
    actor::Actor,
    GameTime,
    level::CleanUp,
    message::Message,
    character::AsCharacter,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
    ammo: u32,
    pub definition: &'static WeaponDefinition,
    pub sender: Option<Sender<Message>>,
}

pub struct WeaponDefinition {
    pub model: &'static str,
    pub shot_sound: &'static str,
    pub ammo: u32,
    pub projectile: ProjectileKind,
    pub shoot_interval: f64,
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
            ammo: 250,
            definition: Self::get_definition(WeaponKind::M4),
            sender: None,
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

        self.definition = Self::get_definition(self.kind);
        self.model.visit("Model", visitor)?;
        self.laser_dot.visit("LaserDot", visitor)?;
        self.offset.visit("Offset", visitor)?;
        self.dest_offset.visit("DestOffset", visitor)?;
        self.last_shot_time.visit("LastShotTime", visitor)?;
        self.owner.visit("Owner", visitor)?;
        self.ammo.visit("Ammo", visitor)?;

        visitor.leave_region()
    }
}

impl Weapon {
    pub fn get_definition(kind: WeaponKind) -> &'static WeaponDefinition {
        match kind {
            WeaponKind::M4 => {
                static DEFINITION: WeaponDefinition = WeaponDefinition {
                    model: "data/models/m4.FBX",
                    shot_sound: "data/sounds/m4_shot.ogg",
                    ammo: 200,
                    projectile: ProjectileKind::Bullet,
                    shoot_interval: 0.15,
                };
                &DEFINITION
            }
            WeaponKind::Ak47 => {
                static DEFINITION: WeaponDefinition = WeaponDefinition {
                    model: "data/models/ak47.FBX",
                    shot_sound: "data/sounds/m4_shot.ogg",
                    ammo: 200,
                    projectile: ProjectileKind::Bullet,
                    shoot_interval: 0.15,
                };
                &DEFINITION
            }
            WeaponKind::PlasmaRifle => {
                static DEFINITION: WeaponDefinition = WeaponDefinition {
                    model: "data/models/plasma_rifle.FBX",
                    shot_sound: "data/sounds/plasma_shot.ogg",
                    ammo: 100,
                    projectile: ProjectileKind::Plasma,
                    shoot_interval: 0.25,
                };
                &DEFINITION
            }
        }
    }

    pub fn new(kind: WeaponKind, resource_manager: &mut ResourceManager, scene: &mut Scene, sender: Sender<Message>) -> Weapon {
        let definition = Self::get_definition(kind);

        let model = resource_manager.request_model(Path::new(definition.model))
            .unwrap()
            .lock()
            .unwrap()
            .instantiate_geometry(scene);

        let laser_dot = scene.graph.add_node(Node::Light(
            LightBuilder::new(LightKind::Point(PointLight::new(0.5)), BaseBuilder::new())
                .with_color(Color::opaque(255, 0, 0))
                .cast_shadows(false)
                .build()));

        let shot_point = scene.graph.find_by_name(model, "Weapon:ShotPoint");

        if shot_point.is_none() {
            println!("Shot point not found!");
        }

        Weapon {
            kind,
            laser_dot,
            model,
            shot_point,
            definition,
            ammo: definition.ammo,
            sender: Some(sender),
            ..Default::default()
        }
    }

    pub fn set_visibility(&self, visibility: bool, graph: &mut Graph) {
        graph.get_mut(self.model)
            .base_mut()
            .set_visibility(visibility);
        graph.get_mut(self.laser_dot)
            .base_mut()
            .set_visibility(visibility);
    }

    pub fn get_model(&self) -> Handle<Node> {
        self.model
    }

    pub fn update(&mut self, scene: &mut Scene, actors: &ActorContainer) {
        self.offset.follow(&self.dest_offset, 0.2);

        self.update_laser_sight(&mut scene.graph, &scene.physics, actors);

        let node = scene.graph.get_mut(self.model);
        node.base_mut().get_local_transform_mut().set_position(self.offset);
        self.shot_position = node.base().get_global_position();
    }

    pub fn get_shot_position(&self, graph: &Graph) -> Vec3 {
        if self.shot_point.is_some() {
            graph.get(self.shot_point)
                .base()
                .get_global_position()
        } else {
            // Fallback
            graph.get(self.model)
                .base()
                .get_global_position()
        }
    }

    pub fn get_shot_direction(&self, graph: &Graph) -> Vec3 {
        graph.get(self.model)
            .base()
            .get_look_vector()
    }

    pub fn get_kind(&self) -> WeaponKind {
        self.kind
    }

    pub fn add_ammo(&mut self, amount: u32) {
        self.ammo += amount;
    }

    fn update_laser_sight(&self, graph: &mut Graph, physics: &Physics, actors: &ActorContainer) {
        let mut laser_dot_position = Vec3::ZERO;
        let model = graph.get(self.model);
        let begin = model.base().get_global_position();
        let end = begin + model.base().get_look_vector().scale(100.0);
        if let Some(ray) = Ray::from_two_points(&begin, &end) {
            let mut result = Vec::new();
            if physics.ray_cast(&ray, RayCastOptions::default(), &mut result) {
                'hit_loop: for hit in result {
                    // Filter hit with owner capsule
                    if let HitKind::Body(body) = hit.kind {
                        for (handle, actor) in actors.pair_iter() {
                            if self.owner == handle && actor.character().body == body {
                                continue 'hit_loop;
                            }
                        }
                    }
                    let offset = hit.normal.normalized().unwrap_or_default().scale(0.2);
                    laser_dot_position = hit.position + offset;
                }
            }
        }

        graph.get_mut(self.laser_dot)
            .base_mut()
            .get_local_transform_mut()
            .set_position(laser_dot_position);
    }

    pub fn get_ammo(&self) -> u32 {
        self.ammo
    }

    pub fn get_owner(&self) -> Handle<Actor> {
        self.owner
    }

    pub fn set_owner(&mut self, owner: Handle<Actor>) {
        self.owner = owner;
    }

    pub fn try_shoot(&mut self, scene: &mut Scene, time: GameTime) -> bool {
        if self.ammo != 0 && time.elapsed - self.last_shot_time >= self.definition.shoot_interval {
            self.ammo -= 1;

            self.offset = Vec3::new(0.0, 0.0, -0.05);
            self.last_shot_time = time.elapsed;

            let position = self.get_shot_position(&scene.graph);

            if let Some(sender) = self.sender.as_ref() {
                sender.send(Message::PlaySound {
                    path: PathBuf::from(self.definition.shot_sound),
                    position,
                }).unwrap();
            }

            true
        } else {
            false
        }
    }
}

impl CleanUp for Weapon {
    fn clean_up(&mut self, scene: &mut Scene) {
        scene.graph.remove_node(self.model);
        scene.graph.remove_node(self.laser_dot);
    }
}

pub struct WeaponContainer {
    pool: Pool<Weapon>
}

impl WeaponContainer {
    pub fn new() -> Self {
        Self {
            pool: Pool::new()
        }
    }

    pub fn add(&mut self, weapon: Weapon) -> Handle<Weapon> {
        self.pool.spawn(weapon)
    }

    pub fn contains(&self, weapon: Handle<Weapon>) -> bool {
        self.pool.is_valid_handle(weapon)
    }

    pub fn free(&mut self, weapon: Handle<Weapon>) {
        self.pool.free(weapon);
    }

    pub fn iter_mut(&mut self) -> PoolIteratorMut<Weapon> {
        self.pool.iter_mut()
    }

    pub fn get(&self, handle: Handle<Weapon>) -> &Weapon {
        self.pool.borrow(handle)
    }

    pub fn get_mut(&mut self, handle: Handle<Weapon>) -> &mut Weapon {
        self.pool.borrow_mut(handle)
    }

    pub fn update(&mut self, scene: &mut Scene, actors: &ActorContainer) {
        for weapon in self.pool.iter_mut() {
            weapon.update(scene, actors)
        }
    }
}

impl Visit for WeaponContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}