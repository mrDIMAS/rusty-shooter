pub mod models {
    pub mod weapons {
        pub const AK47: &str = "data/models/ak47.FBX";
        pub const M4: &str = "data/models/m4.FBX";
        pub const PLASMA_RIFLE: &str = "data/models/plasma_rifle.FBX";
        pub const ROCKET_LAUNCHER: &str = "data/models/Rpg7.FBX";
    }

    pub mod projectiles {
        pub const ROCKET: &str = "data/models/rocket.FBX";
    }

    pub mod items {
        pub const MEDKIT: &str = "data/models/medkit.fbx";
        pub const PLASMA_RIFLE_AMMO: &str = "data/models/yellow_box.FBX";
        pub const AK47_AMMO: &str = "data/models/box_medium.FBX";
        pub const M4_AMMO: &str = "data/models/box_small.FBX";
    }

    pub mod characters {
        pub const MUTANT: &str = "data/models/mutant.FBX";
        pub const PARASITE: &str = "data/models/parasite.FBX";
        pub const MAW: &str = "data/models/maw.fbx";
    }

    pub mod maps {
        pub const DM6: &str = "data/models/dm6.fbx";
    }
}

pub mod textures {
    pub mod particles {
        pub const BULLET: &str = "data/particles/light_01.png";
        pub const SMOKE: &str = "data/particles/smoke_04.tga";
        pub const CIRCLE: &str = "data/particles/circle_05.png";
        pub const STAR: &str = "data/particles/star_09.png";
    }

    pub mod interface {
        pub const CHECK_MARK: &str = "data/ui/check_mark.png";
        pub const CIRCLE: &str = "data/ui/circle.png";

        pub const HEALTH_ICON: &str = "data/ui/health_icon.png";
        pub const AMMO_ICON: &str = "data/ui/ammo_icon.png";
        pub const SHIELD_ICON: &str = "data/ui/shield_icon.png";
        pub const CROSSHAIR: &str = "data/ui/crosshair.tga";
    }
}

pub mod fonts {
    pub const SQUARES_BOLD: &str = "data/ui/SquaresBold.ttf";
}

pub mod animations {
    pub mod mutant {
        pub const IDLE: &str = "data/animations/mutant/idle.fbx";
        pub const WALK: &str = "data/animations/mutant/walk.fbx";
        pub const AIM: &str = "data/animations/mutant/aim.fbx";
        pub const WHIP: &str = "data/animations/mutant/whip.fbx";
        pub const JUMP: &str = "data/animations/mutant/jump.fbx";
        pub const FALLING: &str = "data/animations/mutant/falling.fbx";
        pub const DYING: &str = "data/animations/mutant/dying.fbx";
        pub const DEAD: &str = "data/animations/mutant/dead.fbx";
        pub const HIT_REACTION: &str = "data/animations/mutant/hit_reaction.fbx";
    }

    pub mod parasite {
        pub const IDLE: &str = "data/animations/parasite/idle.fbx";
        pub const WALK: &str = "data/animations/parasite/walk.fbx";
        pub const AIM: &str = "data/animations/parasite/aim.fbx";
        pub const WHIP: &str = "data/animations/parasite/whip.fbx";
        pub const JUMP: &str = "data/animations/parasite/jump.fbx";
        pub const FALLING: &str = "data/animations/parasite/falling.fbx";
        pub const DYING: &str = "data/animations/parasite/dying.fbx";
        pub const DEAD: &str = "data/animations/parasite/dead.fbx";
        pub const HIT_REACTION: &str = "data/animations/parasite/hit_reaction.fbx";
    }

    pub mod maw {
        pub const IDLE: &str = "data/animations/maw/idle.fbx";
        pub const WALK: &str = "data/animations/maw/walk.fbx";
        pub const AIM: &str = "data/animations/maw/aim.fbx";
        pub const WHIP: &str = "data/animations/maw/whip.fbx";
        pub const JUMP: &str = "data/animations/maw/jump.fbx";
        pub const FALLING: &str = "data/animations/maw/falling.fbx";
        pub const DYING: &str = "data/animations/maw/dying.fbx";
        pub const DEAD: &str = "data/animations/maw/dead.fbx";
        pub const HIT_REACTION: &str = "data/animations/maw/hit_reaction.fbx";
    }
}

pub mod sounds {
    pub const HRTF_HRIR: &str = "data/sounds/IRC_1040_C.bin";
    pub const ITEM_PICKUP: &str = "data/sounds/item_pickup.ogg";
    pub const SOUNDTRACK: &str = "data/sounds/Antonio_Bizarro_Berzerker.ogg";

    pub mod shot {
        pub const AK47: &str = "data/sounds/ak47.ogg";
        pub const M4: &str = "data/sounds/m4_shot.ogg";
        pub const PLASMA_RIFLE: &str = "data/sounds/plasma_shot.ogg";
        pub const ROCKET_LAUNCHER: &str = "data/sounds/grenade_launcher_fire.ogg";
    }

    pub mod impact {
        pub const BULLET: &str = "data/sounds/bullet_impact_concrete.ogg";
        pub const ROCKET: &str = "data/sounds/explosion.ogg";
    }

    pub mod footsteps {
        pub const SHOE_STONE: [&str; 4] = [
            "data/sounds/footsteps/FootStep_shoe_stone_step1.wav",
            "data/sounds/footsteps/FootStep_shoe_stone_step2.wav",
            "data/sounds/footsteps/FootStep_shoe_stone_step3.wav",
            "data/sounds/footsteps/FootStep_shoe_stone_step4.wav",
        ];
    }
}
