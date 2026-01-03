use std::f32::consts::PI;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{env, fs};

extern crate sdl2;

use sdl2::audio::{AudioCallback, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

fn keycode_to_hex(key: &Keycode) -> Option<u16> {
    match *key {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

/*
 * ============================================
 * TODO LIST TO BE IMPLEMENTED (INCLUDES FIXES)
 * ============================================
 */

// TODO: Fix bug where finishing program does not result in hanging (?)
// TODO: Pause execution while resizing window
// TODO: Fix bug where incorrectly wraps sprite (seen in pong on clipping through screen)

struct SineWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SineWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for x in out.iter_mut() {
            *x = self.volume * (self.phase * 2.0 * PI).sin();
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

pub fn main() -> Result<(), String> {
    // Get CLI argument for program name
    let args: Vec<String> = env::args().collect();

    let rom = args.get(1).ok_or("Missing ROM argument")?;
    let ipf = args.get(2).ok_or("Missing IPF argument")?;

    // Check to make sure rom exists (otherwise throw error)
    let rom_path: &str = &format!("programs/{}.ch8", rom);
    if !Path::new(rom_path).exists() || !Path::new(rom_path).is_file() {
        return Err(format!("There is no ROM \"{}\" at {}", rom, rom_path));
    }

    // Define number of instructions per frame (propagate error up if encountered)
    let instructions_per_frame: u8 = ipf
        .parse::<u8>()
        .map_err(|_| format!("Enter a valid IPF number"))?;

    // Initialize memory, registers, and stack
    // NOTE: register F is flag register (can be set to 0 or 1)
    let mut registers: [u8; 16] = [0; 16];
    let mut stack: Vec<usize> = Vec::with_capacity(16);
    // NOTE: Storing display state in memory from 0x80 to 0x180
    let mut memory: [u8; 4096] = [0; 4096];
    let mut disp_mem: [bool; 2048] = [false; 2048];

    // Initialize timers
    let mut delay_timer: u8 = 0;
    let mut sound_timer: u8 = 0;

    // Store font in memory
    // NOTE: Only first four bits are used (to make 5x4 bit grid)
    memory[0x50..0x55].copy_from_slice(&[0xF0, 0x90, 0x90, 0x90, 0xF0]); // 0
    memory[0x55..0x5A].copy_from_slice(&[0x20, 0x60, 0x20, 0x20, 0x70]); // 1
    memory[0x5A..0x5F].copy_from_slice(&[0xF0, 0x10, 0xF0, 0x80, 0xF0]); // 2
    memory[0x5F..0x64].copy_from_slice(&[0xF0, 0x10, 0xF0, 0x10, 0xF0]); // 3
    memory[0x64..0x69].copy_from_slice(&[0x90, 0x90, 0xF0, 0x10, 0x10]); // 4
    memory[0x69..0x6E].copy_from_slice(&[0xF0, 0x80, 0xF0, 0x10, 0xF0]); // 5
    memory[0x6E..0x73].copy_from_slice(&[0xF0, 0x80, 0xF0, 0x90, 0xF0]); // 6
    memory[0x73..0x78].copy_from_slice(&[0xF0, 0x10, 0x20, 0x40, 0x40]); // 7
    memory[0x78..0x7D].copy_from_slice(&[0xF0, 0x90, 0xF0, 0x90, 0xF0]); // 8
    memory[0x7D..0x82].copy_from_slice(&[0xF0, 0x90, 0xF0, 0x10, 0xF0]); // 9
    memory[0x82..0x87].copy_from_slice(&[0xF0, 0x90, 0xF0, 0x90, 0x90]); // A
    memory[0x87..0x8C].copy_from_slice(&[0xE0, 0x90, 0xE0, 0x90, 0xE0]); // B
    memory[0x8C..0x91].copy_from_slice(&[0xF0, 0x80, 0x80, 0x80, 0xF0]); // C
    memory[0x91..0x96].copy_from_slice(&[0xE0, 0x90, 0x90, 0x90, 0xE0]); // D
    memory[0x96..0x9B].copy_from_slice(&[0xF0, 0x80, 0xF0, 0x80, 0xF0]); // E
    memory[0x9B..0xA0].copy_from_slice(&[0xF0, 0x80, 0xF0, 0x80, 0x80]); // F

    // Load instructions into memory
    let mem_start: usize = 0x200;
    let contents: Vec<u8> =
        fs::read(rom_path).expect(&format!("Could not read or find chip-8 rom {}", rom_path));
    memory[mem_start..mem_start + contents.len()].copy_from_slice(&contents);

    // Initialize pointers
    let mut pc: usize = mem_start;
    let mut index: usize = 0;

    // Initialize SDL2 subsystems
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let audio_subsystem = sdl_context.audio()?;

    // Initialize audio device
    let mut audio_paused: bool = false;

    let desired_spec = AudioSpecDesired {
        freq: Some(11025),
        channels: Some(1),
        samples: None,
    };

    let beep = audio_subsystem
        .open_playback(None, &desired_spec, |spec| SineWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.25,
        })
        .unwrap();

    // Initialize graphical window
    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 640 * 2, 320 * 2)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    canvas.set_logical_size(64, 32).unwrap();

    let texture_creator = canvas.texture_creator();
    let mut texture: sdl2::render::Texture = texture_creator
        .create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, 64, 32)
        .map_err(|e| e.to_string())?;

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;
    let mut timer_time = Instant::now();

    let mut await_key: bool = false;
    let mut await_op: usize = 0;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                // If FX0A is fetched, store the first released key (assume first pressed)
                // NOTE: filter out keys that are not part of the CHIP-8 keypad
                Event::KeyUp { keycode, .. } => {
                    if await_key {
                        if let Some(key) = keycode_to_hex(&keycode.unwrap_or(Keycode::Space)) {
                            registers[await_op] = key as u8;
                            await_key = false;
                        }
                    }
                }
                _ => {}
            }
        }

        if timer_time.elapsed() > Duration::from_micros(16667) {
            timer_time = Instant::now();

            // Draw canvas (60 Hz refresh rate)
            canvas.clear();
            texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
                for col in 0..64 {
                    for row in 0..32 {
                        let offset = row * pitch + col * 3;
                        let color = if disp_mem[row * 64 + col] {
                            [255, 255, 255]
                        } else {
                            [0, 0, 0]
                        };
                        buffer[offset..offset + 3].copy_from_slice(&color);
                    }
                }
            })?;
            canvas.copy(&texture, None, None)?;
            canvas.present();

            // Update timers
            if sound_timer > 0 {
                beep.resume();
                sound_timer -= 1;
                audio_paused = false;
            } else {
                if !audio_paused {
                    beep.pause();
                    audio_paused = true;
                }
            }

            if delay_timer > 0 {
                delay_timer -= 1;
            }

            let keyboard_state = event_pump.keyboard_state();
            let keycodes_pressed: Vec<Keycode> = keyboard_state
                .pressed_scancodes()
                .into_iter()
                .map(|s| (Keycode::from_scancode(s).unwrap()))
                .collect();

            for _ in 0..instructions_per_frame {
                if await_key {
                    break;
                }

                let opcode: u16 = (memory[pc] as u16) << 8 | (memory[pc + 1] as u16);
                let last_three_nibs: usize = (opcode & 0x0FFF) as usize;
                let last_two_nibs: u8 = (opcode & 0x00FF) as u8;
                let x_nib: usize = ((opcode & 0x0F00) >> 8) as usize;
                let y_nib: usize = ((opcode & 0x00F0) >> 4) as usize;

                pc += 2;

                match (opcode & 0xF000) >> 12 {
                    0x0 => match last_two_nibs {
                        0xE0 => {
                            canvas.set_draw_color(Color::RGB(0, 0, 0));
                            canvas.clear();
                            // memory[0x80..0x180].fill(0);
                            disp_mem.fill(false);
                        }
                        0xEE => pc = stack.pop().unwrap(),
                        _ => {}
                    },
                    0x1 => {
                        // 1NNN
                        pc = last_three_nibs;
                    }
                    0x2 => {
                        // 2NNN
                        stack.push(pc);
                        pc = last_three_nibs;
                    }
                    0x3 => {
                        // 3XNN
                        if registers[x_nib] == last_two_nibs {
                            pc += 2;
                        }
                    }
                    0x4 => {
                        // 4XNN
                        if registers[x_nib] != last_two_nibs {
                            pc += 2;
                        }
                    }
                    0x5 => {
                        // 5XY0
                        if registers[x_nib] == registers[y_nib] {
                            pc += 2;
                        }
                    }
                    0x6 => {
                        // 6XNN
                        registers[x_nib] = last_two_nibs;
                    }
                    0x7 => {
                        // 7XNN
                        registers[x_nib] = registers[x_nib].wrapping_add(last_two_nibs);
                    }
                    0x8 => match opcode & 0x000F {
                        // 8XY[N]
                        0x0 => {
                            registers[x_nib] = registers[y_nib];
                        }
                        0x1 => {
                            registers[x_nib] |= registers[y_nib];
                        }
                        0x2 => {
                            registers[x_nib] &= registers[y_nib];
                        }
                        0x3 => {
                            registers[x_nib] ^= registers[y_nib];
                        }
                        0x4 => {
                            let (sum, flag) = registers[x_nib].overflowing_add(registers[y_nib]);
                            registers[x_nib] = sum;
                            registers[0xF] = flag as u8;
                        }
                        0x5 => {
                            let (sum, flag) = registers[x_nib].overflowing_sub(registers[y_nib]);
                            registers[x_nib] = sum;
                            registers[0xF] = !flag as u8;
                        }
                        0x6 => {
                            let lsb: u8 = registers[x_nib] & 0b1;
                            registers[x_nib] >>= 1;
                            registers[0xF] = lsb;
                        }
                        0x7 => {
                            let (sum, flag) = registers[y_nib].overflowing_sub(registers[x_nib]);
                            registers[x_nib] = sum;
                            registers[0xF] = !flag as u8;
                        }
                        0xE => {
                            let msb: u8 = (registers[x_nib] >> 7) & 0b1;
                            registers[x_nib] <<= 1;
                            registers[0xF] = msb;
                        }

                        _ => {}
                    },
                    0x9 => {
                        // 9XY0
                        if registers[x_nib] != registers[y_nib] {
                            pc += 2;
                        }
                    }
                    0xA => {
                        // ANNN
                        index = last_three_nibs;
                    }
                    0xB => {
                        // BNNN
                        pc = last_three_nibs + registers[0] as usize;
                    }
                    0xC => {
                        // CXNN
                        registers[x_nib] = rand::random::<u8>() & last_two_nibs;
                    }
                    0xD => {
                        // DXYN
                        let x: u16 = (registers[x_nib] % 64) as u16;
                        let y: u16 = (registers[y_nib] % 32) as u16;
                        let height: u16 = opcode & 0x000F;
                        registers[0xF] = 0;
                        for i in 0..height {
                            let row: usize = (y + i) as usize;
                            if row >= 32 {
                                break;
                            }
                            let sprite: u8 = memory[index + (i as usize)];
                            for j in 0..8 {
                                let col: usize = (x + j) as usize;
                                if col >= 64 {
                                    break;
                                }
                                let disp_offset: usize = (row * 64) + col;
                                let prev_bit: bool = disp_mem[disp_offset];
                                let sprite_bit: bool = (sprite >> (7 - j) & 0b1) == 1;
                                disp_mem[disp_offset] ^= sprite_bit;
                                if sprite_bit && prev_bit {
                                    registers[0xF] = 1;
                                }
                            }
                        }
                    }
                    0xE => {
                        match last_two_nibs {
                            /* CHIP-8 layout
                            * 1 2 3 C
                            * 4 5 6 D
                            * 7 8 9 E
                            * A 0 B F
                            */

                            /* Modern layout
                             * 1 2 3 4
                             * Q W E R
                             * A S D F
                             * Z X C V
                             */
                            0x9E => {
                                // EX9E
                                if keycodes_pressed
                                    .iter()
                                    .filter_map(|c| keycode_to_hex(&c))
                                    .any(|c| c == registers[x_nib] as u16)
                                {
                                    pc += 2;
                                }
                            }
                            0xA1 => {
                                // EXA1
                                if !keycodes_pressed
                                    .iter()
                                    .filter_map(|c| keycode_to_hex(&c))
                                    .any(|c| c == registers[x_nib] as u16)
                                {
                                    pc += 2;
                                }
                            }
                            _ => {}
                        }
                    }
                    0xF => match last_two_nibs {
                        // FX[NM]
                        0x07 => {
                            registers[x_nib] = delay_timer;
                        }
                        0x0A => {
                            await_key = true;
                            await_op = x_nib;
                        }
                        0x15 => {
                            delay_timer = registers[x_nib];
                        }
                        0x18 => {
                            sound_timer = registers[x_nib];
                        }
                        0x1E => {
                            index += registers[x_nib] as usize;
                        }
                        0x29 => {
                            index = 0x50 + 5 * registers[x_nib] as usize;
                        }
                        0x33 => {
                            memory[index] = registers[x_nib] / 100;
                            memory[index + 1] = (registers[x_nib] / 10) % 10;
                            memory[index + 2] = registers[x_nib] % 10;
                        }
                        0x55 => {
                            for i in 0..=x_nib {
                                memory[index + i] = registers[i];
                            }
                        }
                        0x65 => {
                            for i in 0..=x_nib {
                                registers[i] = memory[index + i];
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
