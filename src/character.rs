use rg3d::{
    scene::{
        node::Node,
        Scene,
    },
    core::{
        pool::Handle,
        math::vec3::Vec3,
        visitor::{Visit, Visitor, VisitResult},
    },
    physics::{
        rigid_body::RigidBody,
        Physics,
    },
};
use crate::{
    weapon::Weapon,
    level::CleanUp,
    level::GameEvent
};
use std::sync::mpsc::Sender;

pub struct Character {
    pub name: String,
    pub pivot: Handle<Node>,
    pub body: Handle<RigidBody>,
    pub health: f32,
    pub armor: f32,
    pub weapons: Vec<Handle<Weapon>>,
    pub current_weapon: u32,
    pub weapon_pivot: Handle<Node>,
    pub sender: Option<Sender<GameEvent>>,
}

pub trait AsCharacter {
    fn character(&self) -> &Character;
    fn character_mut(&mut self) -> &mut Character;
}

impl Default for Character {
    fn default() -> Self {
        Self {
            name: Default::default(),
            pivot: Handle::NONE,
            body: Handle::NONE,
            health: 100.0,
            armor: 100.0,
            weapons: Vec::new(),
            current_weapon: 0,
            weapon_pivot: Handle::NONE,
            sender: None,
        }
    }
}

impl Visit for Character {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.name.visit("Name", visitor)?;
        self.pivot.visit("Pivot", visitor)?;
        self.body.visit("Body", visitor)?;
        self.health.visit("Health", visitor)?;
        self.armor.visit("Armor", visitor)?;
        self.weapons.visit("Weapons", visitor)?;
        self.current_weapon.visit("CurrentWeapon", visitor)?;
        self.weapon_pivot.visit("WeaponPivot", visitor)?;

        visitor.leave_region()
    }
}

impl CleanUp for Character {
    fn clean_up(&mut self, scene: &mut Scene) {
        scene.remove_node(self.pivot);
        scene.interface_mut().physics.remove_body(self.body);
    }
}

impl Character {
    pub fn get_body(&self) -> Handle<RigidBody> {
        self.body
    }

    pub fn has_ground_contact(&self, physics: &Physics) -> bool {
        let body = physics.borrow_body(self.body);
        for contact in body.get_contacts() {
            if contact.normal.y >= 0.7 {
                return true;
            }
        }
        false
    }

    pub fn get_health(&self) -> f32 {
        self.health
    }

    pub fn get_armor(&self) -> f32 {
        self.armor
    }

    pub fn set_position(&mut self, physics: &mut Physics, position: Vec3) {
        physics.borrow_body_mut(self.get_body()).set_position(position)
    }

    pub fn get_position(&self, physics: &Physics) -> Vec3 {
        physics.borrow_body(self.get_body()).get_position()
    }

    pub fn damage(&mut self, amount: f32) {
        let amount = amount.abs();
        if self.armor > 0.0 {
            self.armor -= amount;
            if self.armor < 0.0 {
                self.health += self.armor;
            }
        } else {
            self.health -= amount;
        }
    }

    pub fn heal(&mut self, amount: f32) {
        self.health += amount.abs();

        if self.health > 150.0 {
            self.health = 150.0;
        }
    }

    pub fn is_dead(&self) -> bool {
        self.health <= 0.0
    }

    pub fn get_weapon_pivot(&self) -> Handle<Node> {
        self.weapon_pivot
    }

    pub fn get_weapons(&self) -> &[Handle<Weapon>] {
        &self.weapons
    }

    pub fn add_weapon(&mut self, weapon: Handle<Weapon>) {
        if let Some(sender) = self.sender.as_ref() {
            for other_weapon in self.weapons.iter() {
                sender.send(GameEvent::ShowWeapon {
                    weapon: *other_weapon,
                    state: false,
                }).unwrap();
            }
        }

        self.current_weapon = self.weapons.len() as u32;
        self.weapons.push(weapon);

        self.request_current_weapon_visible(true);
    }

    pub fn get_current_weapon(&self) -> Handle<Weapon> {
        if let Some(weapon) = self.weapons.get(self.current_weapon as usize) {
            *weapon
        } else {
            Handle::NONE
        }
    }

    fn request_current_weapon_visible(&self, state: bool) {
        if let Some(sender) = self.sender.as_ref() {
            if let Some(current_weapon) = self.weapons.get(self.current_weapon as usize) {
                sender.send(GameEvent::ShowWeapon {
                    weapon: *current_weapon,
                    state,
                }).unwrap()
            }
        }
    }

    pub fn next_weapon(&mut self) {
        if !self.weapons.is_empty() && (self.current_weapon as usize) < self.weapons.len() - 1 {
            self.request_current_weapon_visible(false);

            self.current_weapon += 1;

            self.request_current_weapon_visible(true);
        }
    }

    pub fn prev_weapon(&mut self) {
        if self.current_weapon > 0 {
            self.request_current_weapon_visible(false);

            self.current_weapon -= 1;

            self.request_current_weapon_visible(true);
        }
    }
}
