# What is this?

3d shooter written in Rust. This is huge demo for [rg3d engine](https://github.com/mrDIMAS/rg3d)

## Screenshots

![1](pics/1.jpg?raw=true "Game 1")

![2](pics/2.jpg?raw=true "Game 2")

![3](pics/3.jpg?raw=true "Game 3")

![4](pics/4.jpg?raw=true "Game 4")

![5](pics/5.jpg?raw=true "Game 5")

## What is done already?

- Player 
- Weapons
- Level (q3dm6'ish)
- Stupid bots
- Jump pads (as in Quake 3)
- Items (health, ammo, etc.)

## What will be added soon (tm)

- AI - right now bots are stupid , they just follow you and can't even attack.
- Gameplay - there is not much of gameplay right now - you can walk, jump, shoot (even kill bots, but they won't respawn), collect items. Game modes (deathmatch, capture the flag) are on my list, but since I'm working on engine and the game at the same time I *really* does not have enough time. 

## Notes

Cargo.toml contains hardcoded relative path to engine `rg3d = { path = "../rg3d" }`, so you have to change this or put engine folder near the game folder to get game compile, because it always uses latest rg3d which could be not published on crates.io 
