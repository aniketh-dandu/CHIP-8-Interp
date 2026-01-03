# CHIP 8 Interpreter

[![forthebadge](http://forthebadge.com/images/badges/made-with-rust.svg)](https://forthebadge.com)

### A toy interpeter for the CHIP-8 programming language

<img width="1249" height="682" alt="tetris" src="https://github.com/user-attachments/assets/e6f86f5c-a5cf-4d2c-a566-de131bf8337e" />

## Installation

Clone the repo and follow the usage instructions below. Install sdl2 for the platform of choice (make sure to also download the dev libraries).

## Usage

Place ROMs in the programs folder. A good source of ROMs can be found [here](https://github.com/kripod/chip8-roms). To run the interpeter, run the following command

```bash
cargo run -- <r> <f>
```

where r is the name of the rom *without* a .ch8 or .chip8 extension and where f is the instruction per frame count (between 10-15 is a good value)

The standard Chip-8 keyboard layout and modern mapping is used

A release flag is optional, and a release build can also be made with 

```bash
cargo build --release
```

# Crates

The crates used in this project are
- sdl2
- rand


<img width="1254" height="691" alt="pong" src="https://github.com/user-attachments/assets/1448d698-730e-40a5-a882-1cc3bb9bc1e8" />

