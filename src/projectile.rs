use rg3d::{
    engine::resource_manager::ResourceManager,
    resource::texture::TextureKind,
    scene::{
        sprite::SpriteBuilder,
        Scene,
        SceneInterfaceMut,
        node::Node,
        graph::Graph,
        base::{BaseBuilder, AsBase},
        light::{LightBuilder, LightKind, PointLight}
    }
};
use crate::{
    GameTime,
    effects,
    actor::ActorContainer,
    CollisionGroups,
    character::AsCharacter,
    weapon::{
        Weapon,
        WeaponContainer
    },
    level::CleanUp,
    HandleFromSelf,
};
use std::path::Path;
use rand::Rng;
use rg3d_physics::{
    convex_shape::{ConvexShape, SphereShape},
    RayCastOptions, rigid_body::{RigidBody, CollisionFlags},
    HitKind,
};
use rg3d_core::{
    visitor::{Visit, VisitResult, Visitor},
    pool::{Handle, Pool, PoolIterator},
    color::Color,
    math::{vec3::Vec3, ray::Ray},
};

pub enum ProjectileKind {
    Plasma,
    Bullet,
}

impl ProjectileKind {
    pub fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(ProjectileKind::Plasma),
            1 => Ok(ProjectileKind::Bullet),
            _ => Err(format!("Invalid projectile kind id {}", id))
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            ProjectileKind::Plasma => 0,
            ProjectileKind::Bullet => 1,
        }
    }
}

pub struct Projectile {
    self_handle: Handle<Projectile>,
    kind: ProjectileKind,
    model: Handle<Node>,
    body: Handle<RigidBody>,
    dir: Vec3,
    initial_pos: Vec3,
    speed: f32,
    lifetime: f32,
    rotation_angle: f32,
    ray_based: bool,
    damage: f32,
    owner: Handle<Weapon>,
    initial_velocity: Vec3,
}

impl HandleFromSelf<Projectile> for Projectile {
    fn self_handle(&self) -> Handle<Projectile> {
        self.self_handle
    }
}

impl Default for Projectile {
    fn default() -> Self {
        Self {
            self_handle: Default::default(),
            kind: ProjectileKind::Plasma,
            model: Default::default(),
            dir: Default::default(),
            body: Default::default(),
            speed: 0.0,
            lifetime: 0.0,
            rotation_angle: 0.0,
            ray_based: false,
            damage: 0.0,
            initial_pos: Vec3::ZERO,
            owner: Default::default(),
            initial_velocity: Default::default(),
        }
    }
}

impl Projectile {
    pub fn new(kind: ProjectileKind,
               resource_manager: &mut ResourceManager,
               scene: &mut Scene,
               dir: Vec3,
               position: Vec3,
               owner: Handle<Weapon>,
               initial_velocity: Vec3) -> Self {
        let SceneInterfaceMut { graph, node_rigid_body_map, physics, .. } = scene.interface_mut();

        let (model, body, lifetime, speed, ray_based, damage) = {
            match &kind {
                ProjectileKind::Plasma => {
                    let size = rand::thread_rng().gen_range(0.09, 0.12);

                    let color = Color::opaque(0, 162, 232);
                    let model = graph.add_node(Node::Sprite(SpriteBuilder::new(BaseBuilder::new())
                        .with_size(size)
                        .with_color(color)
                        .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/light_01.png"), TextureKind::R8))
                        .build()));

                    let light = graph.add_node(Node::Light(LightBuilder::new(
                        LightKind::Point(PointLight::new(1.5)), BaseBuilder::new())
                        .with_color(color)
                        .build()));

                    graph.link_nodes(light, model);

                    let mut body = RigidBody::new(ConvexShape::Sphere(SphereShape::new(size)));
                    body.set_gravity(Vec3::ZERO);
                    body.set_position(position);
                    body.collision_group = CollisionGroups::Projectile as u64;
                    // Projectile-Projectile collisions is disabled.
                    body.collision_mask = CollisionGroups::All as u64 & !(CollisionGroups::Projectile as u64);
                    body.collision_flags = CollisionFlags::DISABLE_COLLISION_RESPONSE;

                    (model, physics.add_body(body), 6.0, 0.15, false, 30.0)
                }
                ProjectileKind::Bullet => {
                    (Handle::NONE, Handle::NONE, 0.0, 0.0, true, 20.0)
                }
            }
        };

        if model.is_some() && body.is_some() {
            node_rigid_body_map.insert(model, body);
        }

        Self {
            lifetime,
            body,
            initial_velocity,
            speed,
            rotation_angle: 0.0,
            dir: dir.normalized().unwrap_or(Vec3::UP),
            kind,
            model,
            ray_based,
            damage,
            initial_pos: position,
            owner,
            ..Default::default()
        }
    }

    pub fn is_dead(&self) -> bool {
        self.lifetime <= 0.0
    }

    pub fn update(&mut self,
                  scene: &mut Scene,
                  resource_manager: &mut ResourceManager,
                  actors: &mut ActorContainer,
                  weapons: &WeaponContainer,
                  time: GameTime) {
        let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();

        if self.ray_based {
            let end = self.initial_pos + self.dir.scale(100.0);
            if let Some(ray) = Ray::from_two_points(&self.initial_pos, &end) {
                let mut result = Vec::new();
                if physics.ray_cast(&ray, RayCastOptions::default(), &mut result) {
                    for hit in result.iter() {
                        effects::create_bullet_impact(graph, resource_manager, hit.position);

                        if let HitKind::Body(body) = hit.kind {
                            for actor in actors.iter_mut() {
                                if actor.character().get_body() == body {
                                    let weapon = weapons.get(self.owner);
                                    // Prevent self-damage - this could happen if ray will intersect
                                    // rigid body of owner.
                                    if weapon.get_owner() != actor.self_handle() {
                                        actor.character_mut().damage(self.damage);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            self.lifetime -= time.delta;

            for contact in physics.borrow_body(self.body).get_contacts() {
                let mut owner_contact = false;

                // Check if we got contact with any actor and damage it then.
                for actor in actors.iter_mut() {
                    if contact.body == actor.character().get_body() {
                        // Prevent self-damage.
                        let weapon = weapons.get(self.owner);
                        if weapon.get_owner() != actor.self_handle() {
                            actor.character_mut().damage(self.damage);
                        } else {
                            // In case if we detected contact between owner of this projectile
                            // raise this flag so projectile still will be alive. This prevents
                            // cases when player fires a rocket and it touches player and blows up.
                            owner_contact = true;
                        }
                    }
                }

                if !owner_contact {
                    self.lifetime = 0.0;
                }
            }

            if self.lifetime <= 0.0 {
                effects::create_bullet_impact(graph, resource_manager, self.get_position(graph));
                return;
            }

            if let Node::Sprite(sprite) = graph.get_mut(self.model) {
                sprite.set_rotation(self.rotation_angle);
            }

            let total_velocity = self.initial_velocity + self.dir.scale(self.speed);
            physics.borrow_body_mut(self.body).offset_by(total_velocity);

            self.rotation_angle += 1.5; 
        }
    }

    pub fn get_position(&self, graph: &Graph) -> Vec3 {
        graph.get(self.model).base().get_global_position()
    }
}

impl CleanUp for Projectile {
    fn clean_up(&mut self, scene: &mut Scene) {
        if !self.ray_based {
            let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();

            physics.remove_body(self.body);
            graph.remove_node(self.model);
        }
    }
}

impl Visit for Projectile {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind = self.kind.id();
        kind.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = ProjectileKind::new(kind)?;
        }

        self.self_handle.visit("SelfHandle", visitor)?;
        self.lifetime.visit("Lifetime", visitor)?;
        self.dir.visit("Direction", visitor)?;
        self.speed.visit("Speed", visitor)?;
        self.model.visit("Model", visitor)?;
        self.body.visit("Body", visitor)?;
        self.rotation_angle.visit("RotationAngle", visitor)?;
        self.ray_based.visit("RayBased", visitor)?;
        self.damage.visit("Damage", visitor)?;
        self.initial_velocity.visit("InitialVelocity", visitor)?;

        visitor.leave_region()
    }
}

pub struct ProjectileContainer {
    pool: Pool<Projectile>
}

impl ProjectileContainer {
    pub fn new() -> Self {
        Self {
            pool: Pool::new()
        }
    }

    pub fn add(&mut self, projectile: Projectile) -> Handle<Projectile> {
        let handle = self.pool.spawn(projectile);
        self.pool.borrow_mut(handle).self_handle = handle;
        handle
    }

    pub fn iter(&self) -> PoolIterator<Projectile> {
        self.pool.iter()
    }

    pub fn update(&mut self,
                  scene: &mut Scene,
                  resource_manager: &mut ResourceManager,
                  actors: &mut ActorContainer,
                  weapons: &WeaponContainer,
                  time: GameTime) {
        for projectile in self.pool.iter_mut() {
            projectile.update(scene, resource_manager, actors, weapons, time);
            if projectile.is_dead() {
                projectile.clean_up(scene);
            }
        }

        self.pool.retain(|proj| !proj.is_dead());
    }
}

impl Visit for ProjectileContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}