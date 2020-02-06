use crate::{bot::BotKind, weapon::{
    WeaponKind,
    Weapon
}, actor::Actor, item::{
    ItemKind,
    Item
}, projectile::ProjectileKind, effects::EffectKind, MatchOptions};
use std::path::PathBuf;
use rg3d::{
    core::{
        pool::Handle,
        math::vec3::Vec3
    }
};

#[derive(Debug)]
pub enum Message {
    GiveNewWeapon {
        actor: Handle<Actor>,
        kind: WeaponKind,
    },
    AddBot {
            kind: BotKind,
            position: Vec3,
            name: Option<String>
        },
    RemoveActor {
        actor: Handle<Actor>
    },
    /// Spawns new bot at random spawn point. Selection of spawn point can be based on some
    /// particular heuristic, leading to good selection (like do spawn at a point with least
    /// enemies nearby, which will increase survival probability)
    SpawnBot {
        kind: BotKind
    },
    /// Gives item of specified kind to a given actor. Basically it means that actor will take
    /// item and consume it immediately (heal itself, add ammo, etc.)
    GiveItem {
        actor: Handle<Actor>,
        kind: ItemKind,
    },
    /// Gives specified actor to a given actor. Removes item from level if temporary or deactivates
    /// it for short period of time if it constant.
    PickUpItem {
        actor: Handle<Actor>,
        item: Handle<Item>,
    },
    SpawnItem {
        kind: ItemKind,
        position: Vec3,
        adjust_height: bool,
        lifetime: Option<f32>,
    },
    CreateProjectile {
        kind: ProjectileKind,
        position: Vec3,
        direction: Vec3,
        initial_velocity: Vec3,
        owner: Handle<Weapon>,
    },
    ShootWeapon {
        weapon: Handle<Weapon>,
        initial_velocity: Vec3,
    },
    PlaySound {
        path: PathBuf,
        position: Vec3,
    },
    ShowWeapon {
        weapon: Handle<Weapon>,
        state: bool,
    },
    DamageActor {
        actor: Handle<Actor>,
        /// Actor who damaged target actor, can be Handle::NONE if damage came from environment
        /// or not from any actor.
        who: Handle<Actor>,
        amount: f32,
    },
    CreateEffect {
        kind: EffectKind,
        position: Vec3,
    },
    SpawnPlayer,
    /// HUD listens such events and puts them into queue.
    AddNotification {
        text: String
    },
    /// Removes specified actor and creates new one at random spawn point.
    RespawnActor {
        actor: Handle<Actor>
    },
    /// Save game state to a file. TODO: Add filename field.
    SaveGame,
    /// Loads game state from a file. TODO: Add filename field.
    LoadGame,
    StartNewGame {
        options: MatchOptions
    },
    QuitGame,
    SetMusicVolume {
        volume: f32
    },
}