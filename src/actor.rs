use crate::{
    bot::Bot,
    player::Player,
    character::{
        AsCharacter,
        Character,
    },
    level::{
        UpdateContext,
    },
    message::Message,
};
use rg3d::{
    core::{
        pool::{
            Handle,
            Pool,
            PoolIterator,
            PoolIteratorMut,
            PoolPairIterator,
            PoolPairIteratorMut
        },
        visitor::{
            Visit,
            Visitor,
            VisitResult,
        },
        math::vec3::Vec3
    },
    scene::{
        base::AsBase,
    },
};
use rg3d::scene::Scene;

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

    pub fn can_be_removed(&self) -> bool {
        static_dispatch!(self, can_be_removed,)
    }

    pub fn clean_up(&mut self, scene: &mut Scene) {
        static_dispatch!(self, clean_up, scene)
    }
}

impl AsCharacter for Actor {
    fn character(&self) -> &Character {
        static_dispatch!(self, character,)
    }

    fn character_mut(&mut self) -> &mut Character {
        static_dispatch!(self, character_mut,)
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

// Helper struct that used to hold information about possible target for bots
// it contains all needed information to select suitable target. This is needed
// because of borrowing rules that does not allows to have a mutable reference
// to array element and iterate over array using immutable borrow.
pub struct TargetDescriptor {
    pub handle: Handle<Actor>,
    pub ptr: *const Actor,
    pub health: f32,
    pub position: Vec3,
}

pub struct ActorContainer {
    pool: Pool<Actor>,
    target_descriptors: Vec<TargetDescriptor>
}

impl ActorContainer {
    pub fn new() -> Self {
        Self {
            pool: Default::default(),
            target_descriptors: Default::default()
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

    pub fn count(&self) -> usize {
        self.pool.alive_count()
    }

    pub fn update(&mut self, context: &mut UpdateContext) {
        self.target_descriptors.clear();
        for (handle, actor) in self.pool.pair_iter() {
            self.target_descriptors.push(TargetDescriptor {
                handle,
                ptr: actor,
                health: actor.character().health,
                position: actor.character().position(&context.scene.physics)
            });
        }

        for (handle, actor) in self.pool.pair_iter_mut() {
            let is_dead = actor.character().is_dead();

            match actor {
                Actor::Bot(bot) => bot.update(handle, context, &self.target_descriptors),
                Actor::Player(player) => player.update(context)
            }

            let character = actor.character_mut();

            if !is_dead {
                for (item_handle, item) in context.items.pair_iter() {
                    let pivot = context.scene.graph.get_mut(item.get_pivot());
                    let body = context.scene.physics.borrow_body(character.get_body());
                    let distance = (pivot.base().global_position() - body.get_position()).len();
                    if distance < 1.25 && !item.is_picked_up() {
                        character.sender
                            .as_ref()
                            .unwrap()
                            .send(Message::PickUpItem {
                                actor: handle,
                                item: item_handle,
                            }).unwrap();
                    }
                }
            }

            // Actors can jump on jump pads.
            for jump_pad in context.jump_pads.iter() {
                let body = context.scene.physics.borrow_body_mut(character.get_body());
                let mut push = false;
                for contact in body.get_contacts() {
                    if contact.static_geom == jump_pad.get_shape() {
                        push = true;
                        break;
                    }
                }
                if push {
                    body.set_velocity(jump_pad.get_force());
                }
            }

            if actor.can_be_removed() {
                // Abuse the fact that actor has sender and use it to send message.
                actor.character()
                    .sender
                    .clone()
                    .as_ref()
                    .unwrap()
                    .send(Message::RespawnActor {
                        actor: handle
                    })
                    .unwrap();
            }
        }
    }

    pub fn iter(&self) -> PoolIterator<Actor> {
        self.pool.iter()
    }

    pub fn pair_iter(&self) -> PoolPairIterator<Actor> {
        self.pool.pair_iter()
    }

    pub fn pair_iter_mut(&mut self) -> PoolPairIteratorMut<Actor> {
        self.pool.pair_iter_mut()
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