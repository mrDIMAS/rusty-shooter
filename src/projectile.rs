use rg3d::{
    engine::resource_manager::ResourceManager,
    resource::texture::TextureKind,
    scene::{
        particle_system::{
            ParticleSystemBuilder,
            EmitterBuilder,
            EmitterKind,
            SphereEmitter,
        },
        sprite::SpriteBuilder,
        Scene,
        SceneInterfaceMut,
        node::{
            NodeKind,
            Node,
            NodeBuilder,
        },
        transform::TransformBuilder,
        graph::Graph,
    },
};
use crate::GameTime;
use std::path::Path;
use rand::Rng;
use rg3d_physics::{
    rigid_body::RigidBody,
    convex_shape::{ConvexShape, SphereShape},
};
use rg3d_core::{
    color_gradient::{GradientPoint, ColorGradient},
    visitor::{Visit, VisitResult, Visitor},
    pool::{Handle, Pool, PoolIterator},
    color::Color,
    math::vec3::Vec3,
    numeric_range::NumericRange,
};

pub enum ProjectileKind {
    Plasma,
}

impl ProjectileKind {
    pub fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(ProjectileKind::Plasma),
            _ => Err(format!("Invalid projectile kind id {}", id))
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            ProjectileKind::Plasma => 0,
        }
    }
}

pub struct Projectile {
    kind: ProjectileKind,
    model: Handle<Node>,
    body: Handle<RigidBody>,
    dir: Vec3,
    speed: f32,
    lifetime: f32,
    rotation_angle: f32,
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

        let (model, body, lifetime, speed) = {
            match &kind {
                ProjectileKind::Plasma => {
                    let size = rand::thread_rng().gen_range(0.06, 0.09);

                    let model = Node::new(NodeKind::Sprite(SpriteBuilder::new()
                        .with_size(size)
                        .with_color(Color::opaque(0, 162, 232))
                        .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/light_01.png"), TextureKind::R8))
                        .build()));

                    let mut body = RigidBody::new(ConvexShape::Sphere(SphereShape::new(size)));
                    body.set_gravity(Vec3::zero());
                    body.set_position(position);

                    (model, body, 6.0, 0.2)
                }
            }
        };

        let model = graph.add_node(model);
        let body = physics.add_body(body);

        node_rigid_body_map.insert(model, body);

        Self {
            lifetime,
            body,
            speed,
            rotation_angle: 0.0,
            dir: dir.normalized().unwrap_or(Vec3::up()),
            kind,
            model,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.lifetime <= 0.0
    }

    pub fn update(&mut self, scene: &mut Scene, time: &GameTime) {
        self.lifetime -= time.delta;

        if self.lifetime <= 0.0 {
            return;
        }

        let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();

        let model = graph.get_mut(self.model);
        if let NodeKind::Sprite(sprite) = model.get_kind_mut() {
            sprite.set_rotation(self.rotation_angle);
        }

        physics.borrow_body_mut(self.body).move_by(self.dir.scale(self.speed * time.delta));

        self.rotation_angle += 1.5;
    }

    pub fn get_position(&self, graph: &Graph) -> Vec3 {
        graph.get(self.model).get_global_position()
    }

    pub fn remove_self(&mut self, scene: &mut Scene) {
        let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();

        physics.remove_body(self.body);
        graph.remove_node(self.model);
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

    fn create_impact_particle_system(scene: &mut Scene, resource_manager: &mut ResourceManager, pos: Vec3) {
        let SceneInterfaceMut { graph, .. } = scene.interface_mut();
        NodeBuilder::new(NodeKind::ParticleSystem(
            ParticleSystemBuilder::new()
                .with_acceleration(Vec3::make(0.0, 0.0, 0.0))
                .with_color_over_lifetime_gradient({
                    let mut gradient = ColorGradient::new();
                    gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
                    gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(150, 150, 150, 220)));
                    gradient.add_point(GradientPoint::new(0.85, Color::from_rgba(255, 255, 255, 180)));
                    gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
                    gradient
                })
                .with_emitters(vec![
                    EmitterBuilder::new(EmitterKind::Sphere(SphereEmitter::new(0.01)))
                        .with_max_particles(100)
                        .with_spawn_rate(50)
                        .with_x_velocity_range(NumericRange::new(-0.01, 0.01))
                        .with_y_velocity_range(NumericRange::new(0.02, 0.03))
                        .with_z_velocity_range(NumericRange::new(-0.01, 0.01))
                        .build()
                ])
                .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/smoke_04.tga"), TextureKind::R8))
                .build()))
            .with_lifetime(5.0)
            .with_local_transform(TransformBuilder::new()
                .with_local_position(pos)
                .build())
            .build(graph);
    }

    pub fn update(&mut self, scene: &mut Scene, resource_manager: &mut ResourceManager, time: &GameTime) {
        for projectile in self.pool.iter_mut() {
            let SceneInterfaceMut { graph, physics, .. } = scene.interface_mut();
            let position = projectile.get_position(graph);
            let collided = physics.borrow_body(projectile.body).get_contacts().len() > 0;
            projectile.update(scene, time);
            if collided {
                projectile.lifetime = 0.0;
            }
            if projectile.is_dead() {
                Self::create_impact_particle_system(scene, resource_manager, position);

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