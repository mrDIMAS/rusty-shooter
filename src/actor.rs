use crate::{
    bot::Bot,
    player::Player,
    character::{
        AsCharacter,
        Character,
    },
    LevelUpdateContext,
    level::{
        LevelEntity,
        CleanUp,
    },
    HandleFromSelf,
    item::ItemKind,
    weapon::WeaponKind,
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
};
use rg3d::scene::{
    SceneInterfaceMut,
    base::AsBase,
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

macro_rules! dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Actor::Player(v) => v.$func($($args),*),
            Actor::Bot(v) => v.$func($($args),*),
        }
    };
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

impl HandleFromSelf<Actor> for Actor {
    fn self_handle(&self) -> Handle<Actor> {
        self.character().self_handle()
    }
}

impl ActorContainer {
    pub fn new() -> Self {
        Self {
            pool: Default::default()
        }
    }

    pub fn add(&mut self, actor: Actor) -> Handle<Actor> {
        let handle = self.pool.spawn(actor);
        self.pool.borrow_mut(handle).character_mut().self_handle = handle;
        handle
    }

    pub fn get(&self, actor: Handle<Actor>) -> &Actor {
        self.pool.borrow(actor)
    }

    pub fn get_mut(&mut self, actor: Handle<Actor>) -> &mut Actor {
        self.pool.borrow_mut(actor)
    }

    pub fn update(&mut self, context: &mut LevelUpdateContext) {
        for actor in self.pool.iter_mut() {
            actor.update(context);

            for item in context.items.iter_mut() {
                let SceneInterfaceMut { graph, physics, .. } = context.scene.interface_mut();
                let pivot = graph.get_mut(item.get_pivot());
                let body = physics.borrow_body(actor.character().get_body());
                let distance = (pivot.base().get_global_position() - body.get_position()).len();
                if distance < 1.25 && !item.is_picked_up() {
                    match item.get_kind() {
                        ItemKind::Medkit => actor.character_mut().heal(20.0),
                        ItemKind::Plasma | ItemKind::Ak47Ammo762 | ItemKind::M4Ammo556 => {
                            for weapon in actor.character().get_weapons() {
                                let weapon = context.weapons.get_mut(*weapon);
                                let (weapon_kind, ammo) = match item.get_kind() {
                                    ItemKind::Medkit => continue,
                                    ItemKind::Plasma => (WeaponKind::PlasmaRifle, 20),
                                    ItemKind::Ak47Ammo762 => (WeaponKind::Ak47, 30),
                                    ItemKind::M4Ammo556 => (WeaponKind::M4, 25),
                                };
                                if weapon.get_kind() == weapon_kind {
                                    weapon.add_ammo(ammo);
                                    break;
                                }
                            }
                        }
                    }
                    item.pick_up();
                }
            }

            for jump_pad in context.jump_pads.iter() {
                let physics = context.scene.interface_mut().physics;
                let body = physics.borrow_body_mut(actor.character().get_body());
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

            if actor.character().is_dead() {
                // Detach weapons first so their nodes won't be removed along with pivot.
                for weapon in actor.character().get_weapons() {
                    let weapon = context.weapons.get(*weapon);
                    context.scene.interface_mut().graph.unlink_nodes(weapon.get_model());
                }

                actor.character_mut().clean_up(context.scene);
            }
        }

        self.pool.retain(|actor| !actor.character().is_dead());
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