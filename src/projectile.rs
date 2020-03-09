use rg3d::{
    engine::resource_manager::ResourceManager,
    resource::texture::TextureKind,
    scene::{
        sprite::SpriteBuilder,
        Scene,
        node::Node,
        graph::Graph,
        base::{BaseBuilder, AsBase},
        light::{LightBuilder, LightKind, PointLight},
        transform::TransformBuilder,
    },
    physics::{
        convex_shape::{ConvexShape, SphereShape},
        RayCastOptions, rigid_body::{RigidBody, CollisionFlags},
        HitKind,
    },
    core::{
        visitor::{Visit, VisitResult, Visitor},
        pool::{Handle, Pool, PoolIteratorMut},
        color::Color,
        math::{vec3::Vec3, ray::Ray},
    },
};
use crate::{
    GameTime,
    actor::{
        ActorContainer,
        Actor,
    },
    CollisionGroups,
    character::AsCharacter,
    weapon::{
        Weapon,
        WeaponContainer,
    },
    message::Message,
    effects::EffectKind
};
use std::{
    path::Path,
    sync::mpsc::Sender,
};
use rand::Rng;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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
    kind: ProjectileKind,
    model: Handle<Node>,
    /// Handle of rigid body assigned to projectile. Some projectiles, like grenades,
    /// rockets, plasma balls could have rigid body to detect collisions with
    /// environment. Some projectiles do not have rigid body - they're ray-based -
    /// interaction with environment handled with ray cast.
    body: Handle<RigidBody>,
    dir: Vec3,
    lifetime: f32,
    rotation_angle: f32,
    /// Handle of weapons from which projectile was fired.
    pub owner: Handle<Weapon>,
    initial_velocity: Vec3,
    /// Position of projectile on the previous frame, it is used to simulate
    /// continuous intersection detection from fast moving projectiles.
    last_position: Vec3,
    definition: &'static ProjectileDefinition,
    pub sender: Option<Sender<Message>>,
}

impl Default for Projectile {
    fn default() -> Self {
        Self {
            kind: ProjectileKind::Plasma,
            model: Default::default(),
            dir: Default::default(),
            body: Default::default(),
            lifetime: 0.0,
            rotation_angle: 0.0,
            owner: Default::default(),
            initial_velocity: Default::default(),
            last_position: Default::default(),
            definition: Self::get_definition(ProjectileKind::Plasma),
            sender: None,
        }
    }
}

pub struct ProjectileDefinition {
    damage: f32,
    speed: f32,
    lifetime: f32,
    /// Means that movement of projectile controlled by code, not physics.
    /// However projectile still could have rigid body to detect collisions.
    is_kinematic: bool,
}

impl Projectile {
    pub fn get_definition(kind: ProjectileKind) -> &'static ProjectileDefinition {
        match kind {
            ProjectileKind::Plasma => {
                static DEFINITION: ProjectileDefinition = ProjectileDefinition {
                    damage: 30.0,
                    speed: 0.15,
                    lifetime: 10.0,
                    is_kinematic: true,
                };
                &DEFINITION
            }
            ProjectileKind::Bullet => {
                static DEFINITION: ProjectileDefinition = ProjectileDefinition {
                    damage: 15.0,
                    speed: 0.75,
                    lifetime: 10.0,
                    is_kinematic: true,
                };
                &DEFINITION
            }
        }
    }

    pub fn new(kind: ProjectileKind,
               resource_manager: &mut ResourceManager,
               scene: &mut Scene,
               dir: Vec3,
               position: Vec3,
               owner: Handle<Weapon>,
               initial_velocity: Vec3,
               sender: Sender<Message>) -> Self {
        let definition = Self::get_definition(kind);

        let (model, body) = {
            match &kind {
                ProjectileKind::Plasma => {
                    let size = rand::thread_rng().gen_range(0.09, 0.12);

                    let color = Color::opaque(0, 162, 232);
                    let model = scene.graph.add_node(Node::Sprite(SpriteBuilder::new(BaseBuilder::new())
                        .with_size(size)
                        .with_color(color)
                        .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/light_01.png"), TextureKind::R8))
                        .build()));

                    let light = scene.graph.add_node(Node::Light(LightBuilder::new(
                        LightKind::Point(PointLight::new(1.5)), BaseBuilder::new())
                        .with_color(color)
                        .build()));

                    scene.graph.link_nodes(light, model);

                    let mut body = RigidBody::new(ConvexShape::Sphere(SphereShape::new(size)));
                    body.set_gravity(Vec3::ZERO);
                    body.set_position(position);
                    body.collision_group = CollisionGroups::Projectile as u64;
                    // Projectile-Projectile collisions is disabled.
                    body.collision_mask = CollisionGroups::All as u64 & !(CollisionGroups::Projectile as u64);
                    body.collision_flags = CollisionFlags::DISABLE_COLLISION_RESPONSE;

                    (model, scene.physics.add_body(body))
                }
                ProjectileKind::Bullet => {
                    let model = scene.graph.add_node(Node::Sprite(SpriteBuilder::new(BaseBuilder::new()
                        .with_local_transform(TransformBuilder::new()
                            .with_local_position(position)
                            .build()))
                        .with_size(0.05)
                        .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/light_01.png"), TextureKind::R8))
                        .build()));

                    (model, Handle::NONE)
                }
            }
        };

        if model.is_some() && body.is_some() {
            scene.physics_binder.bind(model, body);
        }

        Self {
            lifetime: definition.lifetime,
            body,
            initial_velocity,
            dir: dir.normalized().unwrap_or(Vec3::UP),
            kind,
            model,
            last_position: position,
            owner,
            definition,
            sender: Some(sender),
            ..Default::default()
        }
    }

    pub fn is_dead(&self) -> bool {
        self.lifetime <= 0.0
    }

    pub fn kill(&mut self) {
        self.lifetime = 0.0;
    }

    pub fn update(&mut self, scene: &mut Scene, actors: &ActorContainer, weapons: &WeaponContainer, time: GameTime) {
        // Fetch current position of projectile.
        let position = if self.body.is_some() {
            scene.physics.borrow_body(self.body).get_position()
        } else {
            scene.graph.get(self.model).base().get_global_position()
        };

        let mut hits: Vec<Hit> = Vec::new();
        let mut effect_position = None;

        // Do ray based intersection tests for every kind of projectiles. This will help to handle
        // fast moving projectiles.
        if let Some(ray) = Ray::from_two_points(&self.last_position, &position) {
            let mut result = Vec::new();
            if scene.physics.ray_cast(&ray, RayCastOptions::default(), &mut result) {
                // List of hits sorted by distance from ray origin.
                'hit_loop: for hit in result.iter() {
                    if let HitKind::Body(body) = hit.kind {
                        for (actor_handle, actor) in actors.pair_iter() {
                            if actor.character().get_body() == body && self.owner.is_some() {
                                let weapon = weapons.get(self.owner);
                                // Ignore intersections with owners of weapon.
                                if weapon.get_owner() != actor_handle {
                                    hits.push(Hit {
                                        actor: actor_handle,
                                        who: weapon.get_owner(),
                                    });

                                    self.kill();
                                    effect_position = Some(hit.position);
                                    break 'hit_loop;
                                }
                            }
                        }
                    } else {
                        self.kill();
                        effect_position = Some(hit.position);
                        break 'hit_loop;
                    }
                }
            }
        }

        // Movement of kinematic projectiles are controlled explicitly.
        if self.definition.is_kinematic {
            let total_velocity = self.initial_velocity + self.dir.scale(self.definition.speed);

            // Special case for projectiles with rigid body.
            if self.body.is_some() {
                for contact in scene.physics.borrow_body(self.body).get_contacts() {
                    let mut owner_contact = false;

                    // Check if we got contact with any actor and damage it then.
                    for (actor_handle, actor) in actors.pair_iter() {
                        if contact.body == actor.character().get_body() && self.owner.is_some() {
                            // Prevent self-damage.
                            let weapon = weapons.get(self.owner);
                            if weapon.get_owner() != actor_handle {
                                hits.push(Hit {
                                    actor: actor_handle,
                                    who: weapon.get_owner(),
                                });
                            } else {
                                // Make sure that projectile won't die on contact with owner.
                                owner_contact = true;
                            }
                        }
                    }

                    if !owner_contact {
                        self.kill();
                        effect_position = Some(contact.position);
                    }
                }

                // Move rigid body explicitly.
                scene.physics.borrow_body_mut(self.body).offset_by(total_velocity);
            } else {
                // We have just model - move it.
                scene.graph.get_mut(self.model)
                    .base_mut()
                    .get_local_transform_mut()
                    .offset(total_velocity);
            }
        }

        if let Node::Sprite(sprite) = scene.graph.get_mut(self.model) {
            sprite.set_rotation(self.rotation_angle);
            self.rotation_angle += 1.5;
        }

        // Reduce initial velocity down to zero over time. This is needed because projectile
        // stabilizes its movement over time.
        self.initial_velocity.follow(&Vec3::ZERO, 0.15);

        self.lifetime -= time.delta;

        if self.lifetime <= 0.0 {
            self.sender.as_ref().unwrap().send(Message::CreateEffect {
                kind: EffectKind::BulletImpact,
                position: effect_position.unwrap_or(self.get_position(&scene.graph)),
            }).unwrap();
        }

        // List of hit actors can contain same actor multiple times in a row because this list could
        // be filled from ray casting as well as from contact information of rigid body, fix this
        // to not damage actor twice or more times with one projectile.
        hits.dedup_by(|a, b| a.actor == b.actor);
        for hit in hits {
            self.sender.as_ref().unwrap().send(Message::DamageActor {
                actor: hit.actor,
                who: hit.who,
                amount: self.definition.damage,
            }).unwrap();
        }

        self.last_position = position;
    }

    pub fn get_position(&self, graph: &Graph) -> Vec3 {
        graph.get(self.model)
            .base()
            .get_global_position()
    }

    fn clean_up(&mut self, scene: &mut Scene) {
        if self.body.is_some() {
            scene.physics.remove_body(self.body);
        }
        if self.model.is_some() {
            scene.graph.remove_node(self.model);
        }
    }
}

struct Hit {
    actor: Handle<Actor>,
    who: Handle<Actor>,
}

impl Visit for Projectile {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind = self.kind.id();
        kind.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = ProjectileKind::new(kind)?;
        }

        self.definition = Self::get_definition(self.kind);
        self.lifetime.visit("Lifetime", visitor)?;
        self.dir.visit("Direction", visitor)?;
        self.model.visit("Model", visitor)?;
        self.body.visit("Body", visitor)?;
        self.rotation_angle.visit("RotationAngle", visitor)?;
        self.initial_velocity.visit("InitialVelocity", visitor)?;
        self.owner.visit("Owner", visitor)?;

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
        self.pool.spawn(projectile)
    }

    pub fn iter_mut(&mut self) -> PoolIteratorMut<Projectile> {
        self.pool.iter_mut()
    }

    pub fn update(&mut self, scene: &mut Scene, actors: &ActorContainer, weapons: &WeaponContainer, time: GameTime) {
        for projectile in self.pool.iter_mut() {
            projectile.update(scene, actors, weapons, time);
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