# What is this?

3d shooter written in Rust and based on [rg3d engine](https://github.com/mrDIMAS/rg3d)

## How to build

Cargo.toml contains hardcoded relative path to engine `rg3d = { path = "../rg3d" }`, so you have to change this or put engine folder near the game folder to get game compile, because it always uses latest rg3d which could be not published on crates.io.

In other words you can do something like this:
```
git clone https://github.com/mrDIMAS/rg3d
git clone https://github.com/mrDIMAS/rusty-shooter
cd rusty-shooter
cargo run --release
```

Or if you're updating to the latest version, do this:
```
cd rg3d
git pull
cd ../rusty-shooter
git pull
cargo run --release
```

## Gameplay video

Keep in mind that it can be different from latest version!

[![Gameplay video](pics/rusty-shooter-youtube.PNG?raw=true "Video")](https://www.youtube.com/watch?v=UDn8ymyXPcI)

## Screenshots

![1](pics/1.jpg?raw=true "Game 1")

![2](pics/2.jpg?raw=true "Game 2")

![3](pics/3.jpg?raw=true "Game 3")

![4](pics/4.jpg?raw=true "Game 4")

![5](pics/5.jpg?raw=true "Game 5")

## Plan

- [x] Player movement
	- [x] Walk
	- [x] Jump
	- [x] Crouch - it does not check if you have enough space to stand up and may push you off level. Also when standing up character jumps.
	- [x] Run
- [ ] Weapons
	- [x] AK47
	- [x] M4
	- [x] Plasma
	- [ ] Grenade launcher. 
	- [x] Rocket launcher.
	- [ ] Lightning gun.
	- [ ] Machine gun.
- [x] Projectiles. More should be added.
	- [x] Bullet
	- [x] Plasma ball
	- [x] Rocket
	- [ ] Grenade
	- [ ] Nail
- [x] Level - shitty version of legendary q3dm6 level is implemented. Good enough for tests, bad gameplay wise.
- [x] Jump pads - works similar as in Quake 3: actor touches jump pad, it shoots you in specified position.
- [x] Items. List should be extended when new weapons or items are added.
	- [x] Health pack
	- [x] AK47 ammo
	- [x] Plasma ammo
	- [x] M4 ammo
- [x] Respawn - player and bots will respawn after death. Still need to think a way of how this will work with game modes.
- [x] Spawn points - done, actors will respawn on points with least amount of enemies nearby.
- [x] Stupid bots - dumb bots that follows you in a straight line are done. Next iteration needed.
- [x] Main menu
	- [x] New game
	- [x] Save game
	- [x] Load game
	- [x] Options
	- [x] Quit
- [x] Options
	- [x] Controls
		- [x] Common key bindings
		- [x] Mouse sensitivity
		- [x] Mouse inversion
		- [x] Reset to defaults
		- [x] Mouse smoothing
		- [x] Camera shaking
		- [ ] Unique key binding
	- [x] Graphics
		- [x] Resolution
		- [ ] Fullscreen - checkbox is not doing anything
		- [x] Spot shadows
		- [x] Soft spot shadows
		- [x] Spot shadows distance
		- [x] Point shadows
		- [x] Soft point shadows
		- [x] Point shadows distance
	- [x] Sound
		- [x] Sound volume
		- [x] Music volume
		- [x] HRTF		
- [x] Save/load - game state can be saved/loaded at any time.
- [x] HUD
	- [x] Ammo
	- [x] Health
	- [x] Armor
	- [ ] Game mode specific score
		- [x] Death match
		- [ ] Team death match
		- [ ] Capture the flag
- [x] Bot whip attack - bots can punch you in the face you stand too close to them.
	- [x] Damage
- [x] Bots animations - more or less done, bots are fully animated and has configured animation machines. This can change if there will be a need for more animations.
- [x] Sparks when projectile hit surface.
- [x] Ability to pickup dropped weapons.
- [x] Drop weapons when actor die.
- [x] Give player some weapon on respawn.
- [x] Events log - simple text-based event log - it shows all significant events - death of an actor, damage, etc.
- [x] Pathfinding - based on navmesh.
- [x] Death zones - places where actor dies immediately (space, death fog, squashed, telefragged, etc) is added 
- [ ] Level editor - some simple level editor would be nice, for now I'll continue use ancient 3ds max 2012. Game items are placed on level using dummies which then are substituded with real items, this works but very uncomfortable because it is not WYSIWYG editor.
- [x] Restyle UI. Main menu also should have some sort of background, not just black void.
- [x] Loading screen - level loads asynchronously now.
- [ ] Environment interaction - its implemented partially - any actor can use jump pads, pick up items. 
- [ ] More bots - there are only three bot kind available, there are a lot of free models on mixamo.com which can be used to add more bots.
- [ ] More levels - currently there is only one level which is boring as fuck.
- [x] Add small interval between bots/player respawn
- [x] Add something like "You died" text on HUD when player dies.
- [ ] Bots AI
	- [x] Vertical aiming
	- [x] Bots walking from item to item and shooting nearby targets
	- [x] Vision frustum for bots - bots can "see" only in front of them.
	- [x] Automatic weapon selection
	- [x] Remove "wall hack" from bots - currently bots can see thru walls and will try to shoot there.
	- [ ] Make behaviour more natural
- [x] Win/loss mechanics 
- [ ] Bots hit reaction - partially done, bots have hit reaction animation but there is still no visual "proof" that is was hit. Some sort of blood splashes should be added as well as hit sound.
- [ ] Improve sound - many events in game still does not have sound. There are plenty of free sources with sounds, this should be used.
	- [x] Step sounds
	- [x] Shot sounds
	- [x] Music
	- [x] Item pickup	
	- [ ] Damage sound
	- [ ] Jump sound
	- [ ] Ambient sound
- [ ] Leader board - game mode specific leader board should be added. 
	- [x] Bind to specific key
	- [x] Time limit
	- [x] Deathmatch
		- [x] Table of Name, Kills, Deaths, K/D Ratio
	- [ ] Team death match
		- [ ] Header with team score: Red Team Frags - Blue Team Frags
		- [ ] Table of Name, Kills, Deaths, K/D Ratio
	- [ ] Capture the flag
		- [ ] Header with team score: Red Team Flags - Blue Team Flags
		- [ ] Table of Name, Kills, Deaths, K/D Ratio		
- [ ] Match options	
	- [x] Time limit
	- [ ] Match type
	- [ ] Map
	- [x] Deathmatch
		- [x] Frag limit
	- [ ] Team deathmatch
		- [ ] Frag limit
	- [ ] Capture the flag
		- [ ] Flag limit
- [ ] Hit marks on surfaces - there is no "visual proof" that projectile has hit surface
- [x] `Deathmatch` game mode - easiest game mode to implement.
	- [x] Count kills per actor
	- [x] Game ends when an actor hits frag or time limit	
	- [x] If timelimit hit, but there are more than one actor with same score - game continues.
- [ ] `Capture the flag` game mode - similar to Q3 game mode is nice to have.
	- [ ] Count flags per team
	- [ ] Game ends when team hits flag limit or time limit
	- [ ] If timelimit hit, but flag score is even - game continues.
- [ ] `Team deathmatch` game mode - again similar to Q3.
	- [ ] Count frags per team
	- [ ] Game ends when team hits frag limit or time limit
	- [ ] If timelimit hit, but frag score is even - game continues.
- [ ] Explosive decorations, this will diverse gameplay a bit.
	- [ ] Barrels
	- [ ] Mine
- [ ] Player's ability to punch enemies in face by weapon.
