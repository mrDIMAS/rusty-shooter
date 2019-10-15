use rg3d::{
    engine::resource_manager::ResourceManager,
    resource::texture::TextureKind,
    scene::{
        sprite::SpriteBuilder,
        Scene,
        SceneInterfaceMut,
        node::{
            NodeKind,
            Node,
        },
        graph::Graph,
    },
};
use crate::{
    GameTime,
    effects,
    actor::ActorContainer
};
use std::path::Path;
use rand::Rng;
use rg3d_physics::{
    convex_shape::{ConvexShape, SphereShape},
    RayCastOptions,
    rigid_body::RigidBody
};
use rg3d_core::{
    visitor::{Visit, VisitResult, Visitor},
    pool::{Handle, Pool, PoolIterator},
    color::Color,
    math::vec3::Vec3,
    math::ray::Ray
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
    kind: ProjectileKind,
    model: Handle<Node>,
    body: Handle<RigidBody>,
    dir: Vec3,
    initial_pos: Vec3,
    speed: f32,
    lifetime: f32,
    rotation_angle: f32,
    ray_based: bool,
    damage: f32
}

impl Default for Projectile {
    fn default() -> Self {
        Self {
            kind: ProjectileKind::Plasma,
            model: Default::default(),
            dir: Default::default(),
            body: Default::default(),
            speed: 0.0,
            lifetime: 0.0,
            rotation_angle: 0.0,
            ray_based: false,
            damage: 0.0,
            initial_pos: Vec3::zero(),
        }
    }
}

impl Projectile {
    pub fn new(kind: ProjectileKind,
               resource_manager: &mut ResourceManager,
               scene: &mut Scene,
               dir: Vec3,
               position: Vec3) -> Self {
        let SceneInterfaceMut { graph, node_rigid_body_map, physics, .. } = scene.interface_mut();

        let (model, body, lifetime, speed, ray_based, damage) = {
            match &kind {
                ProjectileKind::Plasma => {
                    let size = rand::thread_rng().gen_range(0.06, 0.09);

                    let model = graph.add_node(Node::new(NodeKind::Sprite(SpriteBuilder::new()
                        .with_size(size)
                        .with_color(Color::opaque(0, 162, 232))
                        .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/light_01.png"), TextureKind::R8))
                        .build())));

                    let mut body = RigidBody::new(ConvexShape::Sphere(SphereShape::new(size)));
                    body.set_gravity(Vec3::zero());
                    body.set_position(position);

                    (model, physics.add_body(body), 6.0, 0.2, false, 30.0)
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
            speed,
            rotation_angle: 0.0,
            dir: dir.normalized().unwrap_or(Vec3::up()),
            kind,
            model,
            ray_based,
            damage,
            initial_pos: position
        }
    }

    pub fn is_dead(&self) -> bool {
        self.lifetime <= 0.0
    }

    pub fn update(&mut self, scene: &mut Scene, resource_manager: &mut ResourceManager, actors: &mut ActorContainer, time: &GameTime) {
        let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();

        if self.ray_based {
            let end = self.initial_pos + self.dir.scale(100.0);
            if let Some(ray) = Ray::from_two_points(&self.initial_pos, &end) {
                let mut result = Vec::new();
                if physics.ray_cast(&ray, RayCastOptions::default(), &mut result) {
                    effects::create_bullet_impact(graph, resource_manager, result[0].position);

                   // for actor in actors.iter_mut() {
                    //    if actor.
                    //}
                }
            }
        } else {
            self.lifetime -= time.delta;

            if physics.borrow_body(self.body).get_contacts().len() > 0 {
                self.lifetime = 0.0;
            }

            if self.lifetime <= 0.0 {
                effects::create_bullet_impact(graph, resource_manager, self.get_position(graph));
                return;
            }

            let model = graph.get_mut(self.model);
            if let NodeKind::Sprite(sprite) = model.get_kind_mut() {
                sprite.set_rotation(self.rotation_angle);
            }

            physics.borrow_body_mut(self.body).move_by(self.dir.scale(self.speed * time.delta));

            self.rotation_angle += 1.5;
        }
    }

    pub fn get_position(&self, graph: &Graph) -> Vec3 {
        graph.get(self.model).get_global_position()
    }

    pub fn remove_self(&mut self, scene: &mut Scene) {
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

        self.lifetime.visit("Lifetime", visitor)?;
        self.dir.visit("Direction", visitor)?;
        self.speed.visit("Speed", visitor)?;
        self.model.visit("Model", visitor)?;
        self.body.visit("Body", visitor)?;
        self.rotation_angle.visit("RotationAngle", visitor)?;
        self.ray_based.visit("RayBased", visitor)?;
        self.damage.visit("Damage", visitor)?;

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

    pub fn iter(&self) -> PoolIterator<Projectile> {
        self.pool.iter()
    }

    pub fn update(&mut self,
                  scene: &mut Scene,
                  resource_manager: &mut ResourceManager,
                  actors: &mut ActorContainer,
                  time: &GameTime
    ) {
        for projectile in self.pool.iter_mut() {
            let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();
            projectile.update(scene, resource_manager, actors, time);
            if projectile.is_dead() {
                projectile.remove_self(scene);
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