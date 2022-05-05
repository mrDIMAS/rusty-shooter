use crate::{
    bot::Bot, character::Character, level::UpdateContext, message::Message, player::Player,
};
use fyrox::{
    core::{
        algebra::Vector3,
        pool::{Handle, Pool},
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::Scene,
};
use std::ops::{Deref, DerefMut};

#[allow(clippy::large_enum_variant)]
#[derive(Visit)]
pub enum Actor {
    Bot(Bot),
    Player(Player),
}

impl Default for Actor {
    fn default() -> Self {
        Actor::Bot(Default::default())
    }
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Actor::Player(v) => v.$func($($args),*),
            Actor::Bot(v) => v.$func($($args),*),
        }
    };
}

impl Actor {
    pub fn can_be_removed(&self) -> bool {
        static_dispatch!(self, can_be_removed,)
    }

    pub fn clean_up(&mut self, scene: &mut Scene) {
        static_dispatch!(self, clean_up, scene)
    }
}

impl Deref for Actor {
    type Target = Character;

    fn deref(&self) -> &Self::Target {
        match self {
            Actor::Bot(v) => v,
            Actor::Player(v) => v,
        }
    }
}

impl DerefMut for Actor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Actor::Bot(v) => v,
            Actor::Player(v) => v,
        }
    }
}

// Helper struct that used to hold information about possible target for bots
// it contains all needed information to select suitable target. This is needed
// because of borrowing rules that does not allows to have a mutable reference
// to array element and iterate over array using immutable borrow.
pub struct TargetDescriptor {
    pub handle: Handle<Actor>,
    pub health: f32,
    pub position: Vector3<f32>,
}

#[derive(Default, Visit)]
pub struct ActorContainer {
    pool: Pool<Actor>,
    #[visit(skip)]
    target_descriptors: Vec<TargetDescriptor>,
}

impl ActorContainer {
    pub fn new() -> Self {
        Self {
            pool: Default::default(),
            target_descriptors: Default::default(),
        }
    }

    pub fn add(&mut self, actor: Actor) -> Handle<Actor> {
        self.pool.spawn(actor)
    }

    pub fn get(&self, actor: Handle<Actor>) -> &Actor {
        self.pool.borrow(actor)
    }

    pub fn contains(&self, actor: Handle<Actor>) -> bool {
        self.pool.is_valid_handle(actor)
    }

    pub fn get_mut(&mut self, actor: Handle<Actor>) -> &mut Actor {
        self.pool.borrow_mut(actor)
    }

    pub fn free(&mut self, actor_handle: Handle<Actor>) {
        for actor in self.pool.iter_mut() {
            if let Actor::Bot(bot) = actor {
                bot.on_actor_removed(actor_handle);
            }
        }

        self.pool.free(actor_handle);
    }

    pub fn count(&self) -> u32 {
        self.pool.alive_count()
    }

    pub fn update(&mut self, context: &mut UpdateContext) {
        self.target_descriptors.clear();
        for (handle, actor) in self.pool.pair_iter() {
            self.target_descriptors.push(TargetDescriptor {
                handle,
                health: actor.health,
                position: actor.position(&context.scene.graph),
            });
        }

        for (handle, actor) in self.pool.pair_iter_mut() {
            let is_dead = actor.is_dead();

            match actor {
                Actor::Bot(bot) => bot.update(handle, context, &self.target_descriptors),
                Actor::Player(player) => player.update(context),
            }
            if !is_dead {
                for (item_handle, item) in context.items.pair_iter() {
                    let distance = (context.scene.graph[item.get_pivot()].global_position()
                        - actor.position(&context.scene.graph))
                    .norm();
                    if distance < 1.25 && !item.is_picked_up() {
                        actor
                            .sender
                            .as_ref()
                            .unwrap()
                            .send(Message::PickUpItem {
                                actor: handle,
                                item: item_handle,
                            })
                            .unwrap();
                    }
                }
            }

            if actor.can_be_removed() {
                // Abuse the fact that actor has sender and use it to send message.
                actor
                    .sender
                    .clone()
                    .as_ref()
                    .unwrap()
                    .send(Message::RespawnActor { actor: handle })
                    .unwrap();
            }
        }

        self.handle_event(context);
    }

    fn handle_event(&mut self, context: &mut UpdateContext) {
        for actor in self.pool.iter_mut() {
            let mut velocity = None;
            for contact_manifold in context.scene.graph[actor.collider]
                .as_collider()
                .contacts(&context.scene.graph.physics)
            {
                for jump_pad in context.jump_pads.iter() {
                    if contact_manifold.collider2 == jump_pad.collider() {
                        velocity = Some(jump_pad.velocity());
                    }
                }
            }

            if let Some(velocity) = velocity {
                context.scene.graph[actor.get_body()]
                    .as_rigid_body_mut()
                    .set_lin_vel(velocity);
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Actor> {
        self.pool.iter()
    }

    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Actor>, &Actor)> {
        self.pool.pair_iter()
    }

    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (Handle<Actor>, &mut Actor)> {
        self.pool.pair_iter_mut()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Actor> {
        self.pool.iter_mut()
    }
}
