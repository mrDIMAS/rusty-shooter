use rg3d::{
    scene::{
        Scene, SceneInterfaceMut,
        node::{Node, NodeKind},
    },
    engine::resource_manager::ResourceManager,
    resource::texture::TextureKind,
    scene::sprite::SpriteBuilder,
};
use rg3d_core::{
    visitor::{Visit, VisitResult, Visitor},
    pool::{Handle, Pool, PoolIterator},
    color::Color,
    math::vec3::Vec3,
};
use crate::GameTime;
use std::path::Path;

pub enum ProjectileKind {
    Bullet,
}

impl ProjectileKind {
    pub fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(ProjectileKind::Bullet),
            _ => Err(format!("Invalid projectile kind id {}", id))
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            ProjectileKind::Bullet => 0,
        }
    }
}

pub struct Projectile {
    kind: ProjectileKind,
    model: Handle<Node>,
    dir: Vec3,
    speed: f32,
    lifetime: f32,
}

pub struct ProjectileDefinition {
    speed: f32,
    lifetime: u32,
}

impl Default for Projectile {
    fn default() -> Self {
        Self {
            kind: ProjectileKind::Bullet,
            model: Default::default(),
            dir: Default::default(),
            speed: 0.0,
            lifetime: 0.0,
        }
    }
}

impl Projectile {
    pub fn new(kind: ProjectileKind,
               resource_manager: &mut ResourceManager,
               scene: &mut Scene,
               dir: Vec3,
               position: Vec3) -> Self {
        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

        let mut model = {
            match &kind {
                ProjectileKind::Bullet => {
                    Node::new(NodeKind::Sprite(SpriteBuilder::new()
                        .with_size(0.025)
                        .with_color(Color::opaque(255, 255, 0))
                        .with_opt_texture(resource_manager.request_texture(Path::new("data/particles/light_01.png"), TextureKind::R8))
                        .build()))
                }
            }
        };

        model.get_local_transform_mut().set_position(position);

        Self {
            lifetime: 6.0,
            speed: match kind {
                ProjectileKind::Bullet => 25.0,
            },
            dir: dir.normalized().unwrap_or(Vec3::up()),
            kind,
            model: graph.add_node(model),
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

        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

        if let Some(model) = graph.get_mut(self.model) {
            let local_transform = model.get_local_transform_mut();
            local_transform.offset(self.dir.scale(self.speed * time.delta));
        }
    }

    pub fn remove_self(&mut self, scene: &mut Scene) {
        let SceneInterfaceMut { graph, .. } = scene.interface_mut();

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

    pub fn update(&mut self, scene: &mut Scene, time: &GameTime) {
        for projectile in self.pool.iter_mut() {
            projectile.update(scene, time);

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