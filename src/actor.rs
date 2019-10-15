use crate::{
    bot::Bot,
    player::Player,
    GameTime,
    projectile::ProjectileContainer,
};
use rg3d_core::{
    pool::{
        Handle,
        Pool,
        PoolIterator,
        PoolIteratorMut,
    },
    visitor::{
        Visit,
        Visitor,
        VisitResult,
    },
    math::vec3::Vec3,
};
use rg3d::{
    scene::Scene,
    engine::{
        resource_manager::ResourceManager,
    },
};
use std::sync::{Mutex, Arc};
use rg3d_sound::context::Context;
use rg3d_physics::{
    Physics,
    rigid_body::RigidBody
};

pub enum Actor {
    Bot(Bot),
    Player(Player),
}

impl Default for Actor {
    fn default() -> Self {
        Actor::Bot(Default::default())
    }
}

pub trait ActorTrait {
    fn get_body(&self) -> Handle<RigidBody>;

    fn get_health(&self) -> f32;

    fn set_health(&mut self, health: f32);

    fn remove_self(&self, scene: &mut Scene);

    fn update(&mut self,
              sound_context: Arc<Mutex<Context>>,
              resource_manager: &mut ResourceManager,
              scene: &mut Scene,
              time: &GameTime,
              projectiles: &mut ProjectileContainer);

    fn set_position(&mut self, physics: &mut Physics, position: Vec3) {
        physics.borrow_body_mut(self.get_body()).set_position(position)
    }

    fn get_position(&self, physics: &Physics) -> Vec3 {
        physics.borrow_body(self.get_body()).get_position()
    }

    fn damage(&mut self, amount: f32) {
        self.set_health(self.get_health() - amount);
    }

    fn is_dead(&self) -> bool {
        self.get_health() <= 0.0
    }
}

impl Actor {
    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Actor::Player(Default::default())),
            1 => Ok(Actor::Bot(Default::default())),
            _ => Err(format!("Unknown actor kind {}", id))
        }
    }

    pub fn id(&self) -> u32 {
        match self {
            Actor::Player(_) => 0,
            Actor::Bot(_) => 1,
        }
    }
}

/// Helper macros to reduce code bloat.
macro_rules! dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Actor::Player(v) => v.$func($($args),*),
            Actor::Bot(v) => v.$func($($args),*),
        }
    };
}

/// Dispatcher for enum variants.
impl ActorTrait for Actor {
    fn get_body(&self) -> Handle<RigidBody> {
        dispatch!(self, get_body,)
    }

    fn get_health(&self) -> f32 {
        dispatch!(self, get_health,)
    }

    fn set_health(&mut self, health: f32) {
        dispatch!(self, set_health, health)
    }

    fn remove_self(&self, scene: &mut Scene) {
        dispatch!(self, remove_self, scene)
    }

    fn update(&mut self,
                  sound_context: Arc<Mutex<Context>>,
                  resource_manager: &mut ResourceManager,
                  scene: &mut Scene,
                  time: &GameTime,
                  projectiles: &mut ProjectileContainer) {
        dispatch!(self, update, sound_context, resource_manager, scene, time, projectiles)
    }
}

impl Visit for Actor {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id = self.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            *self = Actor::from_id(kind_id)?;
        }

        match self {
            Actor::Player(player) => player.visit("Data", visitor)?,
            Actor::Bot(bot) => bot.visit("Data", visitor)?
        }

        visitor.leave_region()
    }
}

pub struct ActorContainer {
    pool: Pool<Actor>
}

impl ActorContainer {
    pub fn new() -> Self {
        Self {
            pool: Default::default()
        }
    }

    pub fn add(&mut self, actor: Actor) -> Handle<Actor> {
        self.pool.spawn(actor)
    }

    pub fn get(&self, actor: Handle<Actor>) -> &Actor {
        self.pool.borrow(actor)
    }

    pub fn get_mut(&mut self, actor: Handle<Actor>) -> &mut Actor {
        self.pool.borrow_mut(actor)
    }

    pub fn update(&mut self,
                  sound_context: Arc<Mutex<Context>>,
                  resource_manager: &mut ResourceManager,
                  scene: &mut Scene,
                  time: &GameTime,
                  projectiles: &mut ProjectileContainer) {
        for actor in self.pool.iter_mut() {
            actor.update(sound_context.clone(), resource_manager, scene, time, projectiles);

            if actor.is_dead() {
                actor.remove_self(scene);
            }
        }

        self.pool.retain(|actor| !actor.is_dead());
    }

    pub fn iter(&self) -> PoolIterator<Actor> {
        self.pool.iter()
    }

    pub fn iter_mut(&mut self) -> PoolIteratorMut<Actor> {
        self.pool.iter_mut()
    }
}

impl Visit for ActorContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}