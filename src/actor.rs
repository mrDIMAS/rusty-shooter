use crate::{
    bot::Bot,
    player::Player,
    character::{
        AsCharacter,
        Character,
    },
    level::{
        LevelUpdateContext,
        LevelEntity,
    },
    level::GameEvent
};
use rg3d::{
    core::{
        pool::{
            Handle,
            Pool,
            PoolIterator,
            PoolIteratorMut,
            PoolPairIterator
        },
        visitor::{
            Visit,
            Visitor,
            VisitResult,
        },
    },
    scene::{
        SceneInterfaceMut,
        base::AsBase,
    },
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

macro_rules! dispatch {
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
        dispatch!(self, can_be_removed,)
    }
}

impl AsCharacter for Actor {
    fn character(&self) -> &Character {
        dispatch!(self, character,)
    }

    fn character_mut(&mut self) -> &mut Character {
        dispatch!(self, character_mut,)
    }
}

impl LevelEntity for Actor {
    fn update(&mut self, context: &mut LevelUpdateContext) {
        dispatch!(self, update, context)
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

    pub fn free(&mut self, actor: Handle<Actor>) {
        self.pool.free(actor)
    }

    pub fn update(&mut self, context: &mut LevelUpdateContext) {
        for (handle, actor) in self.pool.pair_iter_mut() {
            let is_dead = actor.character().is_dead();

            actor.update(context);

            let character = actor.character_mut();

            if !is_dead {
                for (item_handle, item) in context.items.pair_iter() {
                    let SceneInterfaceMut { graph, physics, .. } = context.scene.interface_mut();
                    let pivot = graph.get_mut(item.get_pivot());
                    let body = physics.borrow_body(character.get_body());
                    let distance = (pivot.base().get_global_position() - body.get_position()).len();
                    if distance < 1.25 && !item.is_picked_up() {
                        character.sender
                            .as_ref()
                            .unwrap()
                            .send(GameEvent::PickUpItem {
                                actor: handle,
                                item: item_handle,
                            }).unwrap();
                    }
                }
            }

            // Actors can jump on jump pads.
            for jump_pad in context.jump_pads.iter() {
                let physics = context.scene.interface_mut().physics;
                let body = physics.borrow_body_mut(character.get_body());
                let mut push = false;
                for contact in body.get_contacts() {
                    if contact.static_geom == jump_pad.get_shape() {
                        push = true;
                        break;
                    }
                }
                if push {
                    body.set_velocity(jump_pad.get_force())
                }
            }

            if actor.can_be_removed() {
                // Abuse the fact that actor has sender and use it to send event.
                if let Some(sender) = actor.character().sender.clone().as_ref() {
                    sender.send(GameEvent::RemoveActor { actor: handle }).unwrap();

                    match actor {
                        Actor::Bot(bot) => {
                            // Spawn bot of same kind, we don't care of preserving state of bot
                            // after death. Leader board still will correctly count score.
                            sender.send(GameEvent::SpawnBot { kind: bot.definition.kind }).unwrap()
                        },
                        Actor::Player(_) => {
                            sender.send(GameEvent::SpawnPlayer).unwrap()
                        },
                    }
                }
            }
        }
    }

    pub fn iter(&self) -> PoolIterator<Actor> {
        self.pool.iter()
    }

    pub fn pair_iter(&self) -> PoolPairIterator<Actor> {
        self.pool.pair_iter()
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