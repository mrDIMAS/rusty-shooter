# What is this?
3d shooter written in Rust. 

## Screenshots
![1](pics/1.png?raw=true "Game 1")

![2](pics/2.png?raw=true "Game 2")

![3](pics/3.png?raw=true "Game 3")

## What is done already?

- Player 
- Weapons
- Simple level

## Notes

Cargo.toml contains hardcoded relative path to engine `rg3d = { path = "../rg3d" }`, so you have to change this or put engine folder near the game folder.

## Dependencies

- rg3d - engine
- glutin - window and OpenGL initialization
