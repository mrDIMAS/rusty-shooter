# What is this?

3d shooter written in Rust and based on [rg3d engine](https://github.com/mrDIMAS/rg3d)

## How to build

Cargo.toml contains hardcoded relative path to engine `rg3d = { path = "../rg3d" }`, so you have to change this or put engine folder near the game folder to get game compile, because it always uses latest rg3d which could be not published on crates.io.
Also make sure rg3d has all its dependent `rg3d-` crates near by of latest version from github. Again this is needed because development of game and engine (with all libraries) are goes in the same time and change could not be published on crates.io.

## Screenshots

![1](pics/1.jpg?raw=true "Game 1")

![2](pics/2.jpg?raw=true "Game 2")

![3](pics/3.jpg?raw=true "Game 3")

![4](pics/4.jpg?raw=true "Game 4")

![5](pics/5.jpg?raw=true "Game 5")

## Plan

- [x] Player movement - player can walk, run, jump, crouch. Crouch is a bugged - it does not check if you have enough space to stand up and may push you off level.
- [x] Weapons - implemented: AK47, M4, Plasma. List should be extended when new weapons are added.
- [x] Projectiles - only 2 for now - Bullet and Plasma ball. More should be added.
- [x] Level - shitty version of legendary q3dm6 level is implemented. Good enough for tests, bad gameplay wise.
- [x] Jump pads - works similar as in Quake 3: actor touches jump pad, it shoots you in specified position.
- [x] Items  - implemented: health pack, AK47 ammo, Plasma ammo, M4 ammo. List should be extended when new weapons or items are added.
- [x] Respawn - player and bots will respawn after death. Still need to think a way of how this will work with game modes.
- [x] Spawn points - done, actors will respawn on points with least amount of enemies nearby.
- [x] Stupid bots - dumb bots that follows you in a straight line are done. Next iteration needed.
- [x] Main menu - five buttons in main menu are fully functional.
- [x] Options - controls, graphics and sound settings are done.
- [x] Save/load - game state can be saved/loaded at any time.
- [x] HUD - is done, it shows armor, ammo, and health.
- [x] Bot whip attack - bots can punch you in the face you stand too close to them.
- [x] Bots animations - more or less done, bots are fully animated and has configured animation machines. This can change if there will be a need for more animations.
- [ ] Level editor - some simple level editor would be nice, for now I'll continue use ancient 3ds max 2012. Game items are placed on level using dummies which then are substituded with real items, this works but very uncomfortable because it is not WYSIWYG editor.
- [ ] Restyle UI - it is boring gray right now. Main menu also should have some sort of background, not just black void.
- [ ] Loading screen - currently game just hangs for 8+ seconds until it load a level, this should be done async.
- [ ] Environment interaction - its implemented partially - any actor can use jump pads, pick up items. 
- [ ] Death zones - places where actor dies immediately should be added (space, death fog, squashed, telefragged, etc).
- [ ] More bots - there are only three bot kind available, there are a lot of free models on mixamo.com which can be used to add more bots.
- [ ] More levels - currently there is only one level which is boring as fuck.
- [ ] AI - bot are very stupid right now and this should be fixed, 
- [ ] Bots hit reaction - partially done, bots have hit reaction animation but there is still no visual "proof" that is was hit. Some sort of blood splashes should be added as well as hit sound.
- [ ] Improve sound - many events in game still does not have sound. There are plenty of free sources with sounds, this should be used.
- [ ] Leader board - game mode specific leader board should be added.
- [ ] Pathfinding - there should be a way to specify navmesh that will be used for navigation. Some crates that can help there - https://crates.io/crates/navmesh - need to check that. 
- [ ] Match options.
- [ ] Hit marks on surfaces - there is no "visual proof" that projectile has hit surface (well there is some shitty "fog", but this was added for tests and should be replaced with something more suitable)
- [ ] `Deathmatch` game mode - easiest game mode to implement.
- [ ] `Capture the flag` game mode - similar to Q3 game mode is nice to have.
- [ ] `Team deathmatch` game mode - again similar to Q3.
- [ ] Events log - simple text-based event log would be cool to implement. It should show all significant events - death of an actor, taken flag, round win, etc.
- [ ] Explosive decorations - explosive barrels, mines, etc. This will diverse gameplay a bit.
- [ ] Grenade launcher. 
- [ ] Rocket launcher.
- [ ] Lightning gun.
- [ ] Machine gun.
- [ ] Player's ability to punch enemies in face by weapon.
- [ ] Ability to pickup dropped weapons - current actor drops all its weapon when die, but this is useless because weapons will float in the air for eternity.
