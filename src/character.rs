use rg3d_core::{
    pool::Handle,
    math::vec3::Vec3,
    visitor::{Visit, Visitor, VisitResult},
};
use rg3d::scene::{
    node::Node,
    Scene,
    SceneInterfaceMut,
};
use rg3d_physics::{
    rigid_body::RigidBody,
    Physics,
};
use crate::{
    weapon::Weapon,
    level::CleanUp,
    HandleFromSelf,
    actor::Actor
};

pub struct Character {
    pub self_handle: Handle<Actor>,
    pub pivot: Handle<Node>,
    pub body: Handle<RigidBody>,
    pub health: f32,
    pub armor: f32,
    pub weapons: Vec<Handle<Weapon>>,
    pub current_weapon: u32,
    pub weapon_pivot: Handle<Node>,
}

impl HandleFromSelf<Actor> for Character {
    fn self_handle(&self) -> Handle<Actor> {
        self.self_handle
    }
}

pub trait AsCharacter {
    fn character(&self) -> &Character;
    fn character_mut(&mut self) -> &mut Character;
}

impl Default for Character {
    fn default() -> Self {
        Self {
            self_handle: Default::default(),
            pivot: Handle::NONE,
            body: Handle::NONE,
            health: 100.0,
            armor: 100.0,
            weapons: Vec::new(),
            current_weapon: 0,
            weapon_pivot: Handle::NONE,
        }
    }
}

impl Visit for Character {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.self_handle.visit("SelfHandle", visitor)?;
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
        let SceneInterfaceMut { physics, .. } = scene.interface_mut();
        physics.remove_body(self.body);
    }
}

impl Character {
    pub fn get_body(&self) -> Handle<RigidBody> {
        self.body
    }

    pub fn get_health(&self) -> f32 {
        self.health
    }

    pub fn set_health(&mut self, health: f32) {
        self.health = health;
    }

    pub fn set_armor(&mut self, armor: f32) {
        self.armor = armor;
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
        self.health -= amount;
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
        self.weapons.push(weapon);
    }

    pub fn get_current_weapon(&self) -> Handle<Weapon> {
        if let Some(weapon) = self.weapons.get(self.current_weapon as usize) {
            *weapon
        } else {
            Handle::NONE
        }
    }

    pub fn next_weapon(&mut self) {
        if !self.weapons.is_empty() && (self.current_weapon as usize) < self.weapons.len() - 1 {
            self.current_weapon += 1;
        }
    }

    pub fn prev_weapon(&mut self) {
        if self.current_weapon > 0 {
            self.current_weapon -= 1;
        }
    }
}
