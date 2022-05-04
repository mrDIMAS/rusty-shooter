use crate::{
    actor::{Actor, ActorContainer},
    effects::EffectKind,
    message::Message,
    weapon::{Weapon, WeaponContainer},
    GameTime,
};
use fyrox::{
    core::{
        algebra::{Matrix3, Point3, UnitQuaternion, Vector3},
        color::Color,
        math::{ray::Ray, Vector3Ext},
        pool::{Handle, Pool},
        rand::Rng,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    rand,
    scene::{
        base::BaseBuilder,
        collider::{ColliderBuilder, ColliderShape, InteractionGroups},
        graph::{physics::RayCastOptions, Graph},
        light::{point::PointLightBuilder, BaseLightBuilder},
        node::Node,
        rigidbody::{RigidBodyBuilder, RigidBodyType},
        sprite::{Sprite, SpriteBuilder},
        transform::TransformBuilder,
        Scene,
    },
};
use std::{collections::HashSet, path::PathBuf, sync::mpsc::Sender};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ProjectileKind {
    Plasma,
    Bullet,
    Rocket,
}

impl ProjectileKind {
    pub fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(ProjectileKind::Plasma),
            1 => Ok(ProjectileKind::Bullet),
            2 => Ok(ProjectileKind::Rocket),
            _ => Err(format!("Invalid projectile kind id {}", id)),
        }
    }

    pub fn id(self) -> u32 {
        match self {
            ProjectileKind::Plasma => 0,
            ProjectileKind::Bullet => 1,
            ProjectileKind::Rocket => 2,
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
    body: Option<Handle<Node>>,
    dir: Vector3<f32>,
    lifetime: f32,
    rotation_angle: f32,
    /// Handle of weapons from which projectile was fired.
    pub owner: Handle<Weapon>,
    initial_velocity: Vector3<f32>,
    /// Position of projectile on the previous frame, it is used to simulate
    /// continuous intersection detection from fast moving projectiles.
    last_position: Vector3<f32>,
    definition: &'static ProjectileDefinition,
    pub sender: Option<Sender<Message>>,
    hits: HashSet<Hit>,
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
            hits: Default::default(),
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
    impact_sound: &'static str,
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
                    impact_sound: "data/sounds/bullet_impact_concrete.ogg",
                };
                &DEFINITION
            }
            ProjectileKind::Bullet => {
                static DEFINITION: ProjectileDefinition = ProjectileDefinition {
                    damage: 15.0,
                    speed: 0.75,
                    lifetime: 10.0,
                    is_kinematic: true,
                    impact_sound: "data/sounds/bullet_impact_concrete.ogg",
                };
                &DEFINITION
            }
            ProjectileKind::Rocket => {
                static DEFINITION: ProjectileDefinition = ProjectileDefinition {
                    damage: 30.0,
                    speed: 0.5,
                    lifetime: 10.0,
                    is_kinematic: true,
                    impact_sound: "data/sounds/explosion.ogg",
                };
                &DEFINITION
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        kind: ProjectileKind,
        resource_manager: ResourceManager,
        scene: &mut Scene,
        dir: Vector3<f32>,
        position: Vector3<f32>,
        owner: Handle<Weapon>,
        initial_velocity: Vector3<f32>,
        sender: Sender<Message>,
        basis: Matrix3<f32>,
    ) -> Self {
        let definition = Self::get_definition(kind);

        let (model, body) = {
            match &kind {
                ProjectileKind::Plasma => {
                    let size = rand::thread_rng().gen_range(0.09..0.12);

                    let color = Color::opaque(0, 162, 232);

                    let model;
                    let collider;
                    let body = RigidBodyBuilder::new(BaseBuilder::new().with_children(&[
                        {
                            model = SpriteBuilder::new(
                                BaseBuilder::new().with_children(&[PointLightBuilder::new(
                                    BaseLightBuilder::new(BaseBuilder::new()).with_color(color),
                                )
                                .with_radius(1.5)
                                .build(&mut scene.graph)]),
                            )
                            .with_size(size)
                            .with_color(color)
                            .with_texture(
                                resource_manager.request_texture("data/particles/light_01.png"),
                            )
                            .build(&mut scene.graph);
                            model
                        },
                        {
                            collider = ColliderBuilder::new(BaseBuilder::new())
                                .with_shape(ColliderShape::ball(size))
                                .build(&mut scene.graph);
                            collider
                        },
                    ]))
                    .with_body_type(RigidBodyType::KinematicPositionBased)
                    .build(&mut scene.graph);

                    (model, Some(body))
                }
                ProjectileKind::Bullet => {
                    let model = SpriteBuilder::new(
                        BaseBuilder::new().with_local_transform(
                            TransformBuilder::new()
                                .with_local_position(position)
                                .build(),
                        ),
                    )
                    .with_size(0.05)
                    .with_texture(resource_manager.request_texture("data/particles/light_01.png"))
                    .build(&mut scene.graph);

                    (model, None)
                }
                ProjectileKind::Rocket => {
                    let resource = resource_manager
                        .request_model("data/models/rocket.FBX")
                        .await
                        .unwrap();
                    let model = resource.instantiate_geometry(scene);
                    scene.graph[model]
                        .local_transform_mut()
                        .set_rotation(UnitQuaternion::from_matrix(&basis))
                        .set_position(position);
                    let light = PointLightBuilder::new(
                        BaseLightBuilder::new(BaseBuilder::new())
                            .with_color(Color::opaque(255, 127, 0)),
                    )
                    .with_radius(1.5)
                    .build(&mut scene.graph);
                    scene.graph.link_nodes(light, model);
                    (model, None)
                }
            }
        };

        Self {
            lifetime: definition.lifetime,
            body,
            initial_velocity,
            dir: dir.try_normalize(std::f32::EPSILON).unwrap_or(Vector3::y()),
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

    pub fn update(
        &mut self,
        scene: &mut Scene,
        actors: &ActorContainer,
        weapons: &WeaponContainer,
        time: GameTime,
    ) {
        // Fetch current position of projectile.
        let position = if let Some(body) = self.body.as_ref() {
            scene.graph[*body].global_position()
        } else {
            scene.graph[self.model].global_position()
        };

        let mut effect_position = None;

        // Do ray based intersection tests for every kind of projectiles. This will help to handle
        // fast moving projectiles.
        let ray = Ray::from_two_points(self.last_position, position);
        let mut query_buffer = Vec::default();
        scene.graph.physics.cast_ray(
            RayCastOptions {
                ray_origin: Point3::from(ray.origin),
                ray_direction: ray.origin,
                max_len: ray.dir.norm(),
                groups: InteractionGroups::default(),
                sort_results: true,
            },
            &mut query_buffer,
        );

        // List of hits sorted by distance from ray origin.
        'hit_loop: for hit in query_buffer.iter() {
            let collider = scene.graph[hit.collider].as_collider();
            let body = collider.parent();

            if matches!(collider.shape(), ColliderShape::Trimesh(_)) {
                self.kill();
                effect_position = Some(hit.position.coords);
                break 'hit_loop;
            } else {
                for (actor_handle, actor) in actors.pair_iter() {
                    if actor.get_body() == body && self.owner.is_some() {
                        let weapon = &weapons[self.owner];
                        // Ignore intersections with owners of weapon.
                        if weapon.owner() != actor_handle {
                            self.hits.insert(Hit {
                                actor: actor_handle,
                                who: weapon.owner(),
                            });

                            self.kill();
                            effect_position = Some(hit.position.coords);
                            break 'hit_loop;
                        }
                    }
                }
            }
        }

        // Movement of kinematic projectiles are controlled explicitly.
        if self.definition.is_kinematic {
            let total_velocity = self.dir.scale(self.definition.speed);

            // Special case for projectiles with rigid body.
            if let Some(body) = self.body.as_ref() {
                // Move rigid body explicitly.
                scene.graph[*body]
                    .local_transform_mut()
                    .offset(total_velocity);
            } else {
                // We have just model - move it.
                scene.graph[self.model]
                    .local_transform_mut()
                    .offset(total_velocity);
            }
        }

        if let Some(sprite) = scene.graph[self.model].cast_mut::<Sprite>() {
            sprite.set_rotation(self.rotation_angle);
            self.rotation_angle += 1.5;
        }

        // Reduce initial velocity down to zero over time. This is needed because projectile
        // stabilizes its movement over time.
        self.initial_velocity.follow(&Vector3::default(), 0.15);

        self.lifetime -= time.delta;

        if self.lifetime <= 0.0 {
            let pos = effect_position.unwrap_or_else(|| self.get_position(&scene.graph));

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::CreateEffect {
                    kind: EffectKind::BulletImpact,
                    position: pos,
                })
                .unwrap();

            self.sender
                .as_ref()
                .unwrap()
                .send(Message::PlaySound {
                    path: PathBuf::from(self.definition.impact_sound),
                    position: pos,
                    gain: 1.0,
                    rolloff_factor: 4.0,
                    radius: 3.0,
                })
                .unwrap();
        }

        for hit in self.hits.drain() {
            self.sender
                .as_ref()
                .unwrap()
                .send(Message::DamageActor {
                    actor: hit.actor,
                    who: hit.who,
                    amount: self.definition.damage,
                })
                .unwrap();
        }

        self.last_position = position;
    }

    pub fn get_position(&self, graph: &Graph) -> Vector3<f32> {
        graph[self.model].global_position()
    }

    fn clean_up(&mut self, scene: &mut Scene) {
        if let Some(body) = self.body.as_ref() {
            scene.graph.remove_node(*body);
        } else {
            if self.model.is_some() {
                scene.graph.remove_node(self.model);
            }
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
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
    pool: Pool<Projectile>,
}

impl ProjectileContainer {
    pub fn new() -> Self {
        Self { pool: Pool::new() }
    }

    pub fn add(&mut self, projectile: Projectile) -> Handle<Projectile> {
        self.pool.spawn(projectile)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Projectile> {
        self.pool.iter_mut()
    }

    pub fn update(
        &mut self,
        scene: &mut Scene,
        actors: &ActorContainer,
        weapons: &WeaponContainer,
        time: GameTime,
    ) {
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
