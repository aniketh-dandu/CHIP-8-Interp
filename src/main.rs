use std::cmp::min;
use std::f32::consts::PI;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{env, fs, process};

extern crate sdl2;

use sdl2::audio::{AudioCallback, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;

fn u8_to_input_ascii(num: &u8) -> u8 {
    match *num {
        0 => 120,
        1 => 49,
        2 => 50,
        3 => 51,
        4 => 113,
        5 => 119,
        6 => 101,
        7 => 97,
        8 => 115,
        9 => 100,
        10 => 122,
        11 => 99,
        12 => 52,
        13 => 114,
        14 => 102,
        15 => 118,
        _ => {
            return 0;
        }
    }
}

/*
 * ============================================
 * TODO LIST TO BE IMPLEMENTED (INCLUDES FIXES)
 * ============================================
 */

// TODO: Fix bug where finishing program does not result in hanging (?)
// TODO: Pause execution while resizing window
// TODO: Add fading effect on removed pixels (TBD)
// TODO: Lower memory consumption

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
    assert_eq!(args.len(), 3, "Please enter two valid arguments");

    let rom_path: &str = &format!("programs/{}.ch8", &args[1]);
    if !Path::new(rom_path).exists() || !Path::new(rom_path).is_file() {
        println!(
            "There is no such rom file at {}\n Please enter a valid rom path",
            rom_path
        );
        process::exit(0);
    }

    // Define number of instructions per frame
    let instructions_per_frame: u32 =
        u32::from_str_radix(&args[2], 10).expect("Enter a valid IPF number");

    // Initialize registers, pointers, and memory
    let mut pc: usize = 0x200;
    let mut index: usize = 0;

    // NOTE: register F is flag register (can be set to 0 or 1)
    let mut registers: [u8; 16] = [0; 16];
    let mut stack: Vec<usize> = vec![];
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
    let mut mem_start: usize = 0x200;
    let contents: Vec<u8> =
        fs::read(rom_path).expect(&format!("Could not read or find chip-8 rom {}", rom_path));
    for byte in &contents {
        memory[mem_start] = *byte;
        mem_start += 1;
    }

    // Initialize SDL2 subsystems
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let audio_subsystem = sdl_context.audio()?;

    // Initialize audio device
    let mut audio_paused: bool = false;

    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
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
                Event::KeyUp { keycode, .. } => {
                    if await_key {
                        registers[await_op] = keycode.unwrap() as u8;
                        await_key = false;
                    }
                }
                _ => {}
            }
        }

        if timer_time.elapsed() > Duration::from_micros(16667) {
            timer_time = Instant::now();

            // Draw canvas (60 Hz refresh rate)
            canvas.clear();
            for col in 0..64 {
                for row in 0..32 {
                    if disp_mem[(col + row * 64) as usize] {
                        canvas.set_draw_color(Color::RGB(255, 255, 255));
                    } else {
                        canvas.set_draw_color(Color::RGB(0, 0, 0));
                    }
                    canvas
                        .draw_point(Point::new(col as i32, row as i32))
                        .unwrap();
                }
            }
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
            let keycodes_pressed: Vec<u32> = keyboard_state
                .pressed_scancodes()
                .into_iter()
                .map(|s| (Keycode::from_scancode(s).unwrap()) as u32)
                .collect();

            for _ in 0..instructions_per_frame {
                if await_key {
                    break;
                }

                let opcode: String = format!(
                    "{:04X}",
                    ((memory[pc] as u16) << 8 | (memory[pc + 1] as u16))
                );
                let nibbles_char: Vec<char> = opcode.chars().collect();
                let nibbles_usize: Vec<usize> = opcode
                    .chars()
                    .map(|c| c.to_digit(16).unwrap() as usize)
                    .collect();
                let nibble_last_three = usize::from_str_radix(&opcode[1..], 16).unwrap();
                let nibble_last_two = u8::from_str_radix(&opcode[2..], 16).unwrap();

                pc += 2;

                match nibbles_char.first().expect("No opcode found!") {
                    '0' => match opcode.as_str() {
                        "00E0" => {
                            canvas.set_draw_color(Color::RGB(0, 0, 0));
                            canvas.clear();
                            disp_mem = [false; 2048];
                        }
                        "00EE" => {
                            pc = stack.pop().unwrap();
                        }
                        _ => {
                            println!("Instruction not found!");
                            println!("{}", opcode);
                        }
                    },
                    '1' => {
                        // 1NNN
                        pc = nibble_last_three;
                    }
                    '2' => {
                        // 2NNN
                        stack.push(pc);
                        pc = nibble_last_three;
                    }
                    '3' => {
                        // 3XNN
                        if registers[nibbles_usize[1]] == nibble_last_two {
                            pc += 2;
                        }
                    }
                    '4' => {
                        // 4XNN
                        if registers[nibbles_usize[1]] != nibble_last_two {
                            pc += 2;
                        }
                    }
                    '5' => {
                        // 5XY0
                        if registers[nibbles_usize[1]] == registers[nibbles_usize[2]] {
                            pc += 2;
                        }
                    }
                    '6' => {
                        // 6XNN
                        // Set register X to NN
                        registers[nibbles_usize[1]] = nibble_last_two;
                    }
                    '7' => {
                        // 7XNN
                        // Add NN to register X
                        registers[nibbles_usize[1]] =
                            registers[nibbles_usize[1]].wrapping_add(*&nibble_last_two);
                    }
                    '8' => match nibbles_char.last().expect("Opcode not found") {
                        // 8XY[N]
                        '0' => {
                            registers[nibbles_usize[1]] = registers[nibbles_usize[2]];
                        }
                        '1' => {
                            registers[nibbles_usize[1]] |= registers[nibbles_usize[2]];
                        }
                        '2' => {
                            registers[nibbles_usize[1]] &= registers[nibbles_usize[2]];
                        }
                        '3' => {
                            registers[nibbles_usize[1]] ^= registers[nibbles_usize[2]];
                        }
                        '4' => {
                            let (sum, flag) = registers[nibbles_usize[1]]
                                .overflowing_add(registers[nibbles_usize[2]]);
                            registers[nibbles_usize[1]] = sum;
                            registers[0xF] = flag as u8;
                        }
                        '5' => {
                            let (sum, flag) = registers[nibbles_usize[1]]
                                .overflowing_sub(registers[nibbles_usize[2]]);
                            registers[nibbles_usize[1]] = sum;
                            registers[0xF] = !flag as u8;
                        }
                        '6' => {
                            let lsb: u8 = registers[nibbles_usize[1]] & 0b1;
                            registers[nibbles_usize[1]] >>= 1;
                            registers[0xF] = lsb;
                        }
                        '7' => {
                            let (sum, flag) = registers[nibbles_usize[2]]
                                .overflowing_sub(registers[nibbles_usize[1]]);
                            registers[nibbles_usize[1]] = sum;
                            registers[0xF] = !flag as u8;
                        }
                        'E' => {
                            let msb: u8 = (registers[nibbles_usize[1]] >> 7) & 0b1;
                            registers[nibbles_usize[1]] <<= 1;
                            registers[0xF] = msb;
                        }
                        _ => {}
                    },
                    '9' => {
                        // 9XY0
                        if registers[nibbles_usize[1]] != registers[nibbles_usize[2]] {
                            pc += 2;
                        }
                    }
                    'A' => {
                        // ANNN
                        // Set index to NNN
                        index = nibble_last_three;
                    }
                    'B' => {
                        // BNNN
                        pc = nibble_last_three + registers[0] as usize;
                    }
                    'C' => {
                        // CXNN
                        registers[nibbles_usize[1]] = rand::random::<u8>() & nibble_last_two;
                    }
                    'D' => {
                        // DXYN
                        let x: u8 = registers[nibbles_usize[1]] % 64;
                        let y: u8 = registers[nibbles_usize[2]] % 32;
                        let height: u16 = nibbles_usize[3] as u16;
                        registers[0xF] = 0;
                        for i in 0..height {
                            let row: usize = min(y as u16 + i, 31) as usize;
                            let sprite: u8 = memory[index + (i as usize)];
                            for j in 0..8 {
                                let col: usize = min((x + j) as usize, 63);
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
                    'E' => {
                        match &opcode[2..] {
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
                            "9E" => {
                                // EX9E
                                if keycodes_pressed.iter().any(|&c| {
                                    c == u8_to_input_ascii(&registers[nibbles_usize[1]]) as u32
                                }) {
                                    pc += 2;
                                }
                            }
                            "A1" => {
                                // EXA1
                                if !keycodes_pressed.iter().any(|&c| {
                                    c == u8_to_input_ascii(&registers[nibbles_usize[1]]) as u32
                                }) {
                                    pc += 2;
                                }
                            }
                            _ => {}
                        }
                    }
                    'F' => match &opcode[2..] {
                        // FX[NM]
                        "07" => {
                            registers[nibbles_usize[1]] = delay_timer;
                        }
                        "0A" => {
                            await_key = true;
                            await_op = nibbles_usize[1];
                        }
                        "15" => {
                            delay_timer = registers[nibbles_usize[1]];
                        }
                        "18" => {
                            sound_timer = registers[nibbles_usize[1]];
                        }
                        "1E" => {
                            index += registers[nibbles_usize[1]] as usize;
                        }
                        "29" => {
                            index = 0x50 + 5 * registers[nibbles_usize[1]] as usize;
                        }
                        "33" => {
                            let num_str = format!("{:0>3}", registers[nibbles_usize[1]]);
                            for i in 0..3 {
                                memory[index + i] =
                                    num_str.chars().nth(i).unwrap().to_digit(10).unwrap() as u8;
                            }
                        }
                        "55" => {
                            for i in 0..=nibbles_usize[1] {
                                memory[index + i] = registers[i];
                            }
                        }
                        "65" => {
                            for i in 0..=nibbles_usize[1] {
                                registers[i] = memory[index + i];
                            }
                        }

                        _ => {}
                    },
                    _ => {
                        println!("Opcode not implemented: {}", opcode);
                        process::exit(1);
                    }
                }
            }
        }
    }

    Ok(())
}
