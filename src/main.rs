use std::{fs, process};

extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use std::time::{Duration, Instant};

// Define number of instructions per frame
const IPF: i32 = 500;

// Define path to ROM
const PROGRAM_PATH: &str = "programs/3-corax+.ch8";

// TODO: Add unit test for u8 helper functions
// TODO: Finish implementing opcodes
// TODO: Test on more programs
// TODO: Split opcode and draw loop onto separate threads (?)

fn u8_to_bits(num: u8) -> [bool; 8] {
    let mut bitarray: [bool; 8] = [false; 8];
    for i in 0..8 {
        bitarray[7 - i] = ((num >> i) & 1) == 1;
    }
    return bitarray;
}

fn bits_to_num(num: &[bool]) -> u32 {
    let mut ret_val: u32 = 0;
    for (i, bit) in num.iter().rev().enumerate() {
        ret_val += match bit {
            true => 1 << i,
            false => 0,
        };
    }
    return ret_val;
}

fn u8_to_hex(number: u8) -> String {
    let num: [bool; 8] = u8_to_bits(number);
    let num_len = num.len();
    let num_nibbles = num_len / 4;
    let remainder = num_len % 4;
    let mut ret_str = String::with_capacity(num_len);
    for i in 0..num_nibbles {
        let hex_str = bits_to_num(&num[(4 * i)..(4 * (i + 1))]);
        ret_str.push_str(format!("{:X}", hex_str).as_str());
    }
    if remainder != 0 {
        let remain_num = bits_to_num(&num[4 * num_nibbles..]);
        ret_str.push_str(format!("{:X}", remain_num).as_str());
    }
    return ret_str;
}

fn add_u8_with_overflow(num1: &u8, num2: &u8) -> u8 {
    return ((*num1 as u16 + *num2 as u16) % 256) as u8;
}

pub fn main() -> Result<(), String> {
    // Initialize registers, pointers, and memory
    let mut pc: usize = 0x200;
    let mut index: usize = 0;

    // NOTE: register F is flag register (can be set to 0 or 1)
    let mut registers: [u8; 16] = [0; 16];
    let mut stack: Vec<usize> = vec![];
    let mut memory: [u8; 4096] = [0; 4096];
    let mut disp_mem: [bool; 2048] = [false; 2048];

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
    let contents: Vec<u8> = fs::read(PROGRAM_PATH).expect("Could not read chip 8 program");
    for byte in &contents {
        memory[mem_start] = *byte;
        mem_start += 1;
    }

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 640, 320)
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

    let mut instruction_count: i32 = 0;
    let mut throttle: bool = false;
    let mut start_time = Instant::now();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        if !throttle {
            instruction_count += 1;

            let opcode: String = format!("{}{}", u8_to_hex(memory[pc]), u8_to_hex(memory[pc + 1]));
            // println!("{}", opcode);
            // let start_nib: char = opcode.chars().next().unwrap();
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
                    // Jump to addr (set program counter)
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
                    // registers[nibbles_usize[1]] += nibble_last_two;
                    registers[nibbles_usize[1]] =
                        add_u8_with_overflow(&registers[nibbles_usize[1]], &nibble_last_two);
                }
                '8' => match nibbles_char.last().expect("Opcode not found") {
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
                        continue;
                    }
                    '5' => {
                        continue;
                    }
                    '6' => {
                        continue;
                    }
                    '7' => {
                        continue;
                    }
                    'E' => {
                        continue;
                    }
                    _ => {}
                },
                '9' => {
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
                    pc = nibble_last_three + registers[0] as usize;
                }
                'C' => {
                    registers[nibbles_usize[1]] = rand::random::<u8>() & nibble_last_two;
                }
                // TODO: Fix timing to remove jittering
                'D' => {
                    // DXYN
                    let x: u8 = registers[nibbles_usize[1]];
                    let y: u8 = registers[nibbles_usize[2]];
                    let height: u16 = nibbles_usize[3] as u16;
                    registers[0xF] = 0;
                    for i in 0..height {
                        let row: u16 = (y % 32) as u16 + i;
                        let sprite: [bool; 8] = u8_to_bits(memory[index + (i as usize)]);
                        for j in 0..8 {
                            let col: u16 = (x % 64) as u16 + j as u16;
                            let disp_offset: usize = ((row * 64) + col) as usize;
                            let prev_bit: bool = disp_mem[disp_offset];
                            if sprite[j as usize] {
                                if prev_bit {
                                    disp_mem[disp_offset] = false;
                                    registers[0xF] = 1;
                                    canvas.set_draw_color(Color::RGB(0, 0, 0));
                                } else {
                                    disp_mem[disp_offset] = true;
                                    canvas.set_draw_color(Color::RGB(255, 255, 255));
                                }
                                canvas
                                    .draw_point(Point::new(col as i32, row as i32))
                                    .unwrap();
                            }
                        }
                    }
                }
                'F' => match &opcode[2..] {
                    "07" => {
                        // Timer
                    }
                    "0A" => {
                        // Key Press
                    }
                    "15" => {
                        // Delay Timer
                    }
                    "18" => {
                        // Sound Timer
                    }
                    "1E" => {
                        index += registers[nibbles_usize[1]] as usize;
                    }
                    "29" => {
                        index = memory[0x50 + 5 * registers[nibbles_usize[1]] as usize] as usize;
                    }
                    "33" => {
                        let num_str = format!("{:0>3}", registers[nibbles_usize[1]]);
                        for i in 0..3 {
                            memory[index + i] = num_str.chars().nth(i).unwrap() as u8;
                        }
                    }
                    "65" => {
                        for i in 0..16 {
                            registers[i] = memory[index + i];
                        }
                    }

                    _ => {}
                },
                _ => {
                    println!("{}", opcode);
                    process::exit(1);
                    // break;
                }
            }
        }

        if instruction_count >= IPF {
            throttle = true;
            instruction_count = 0;
        }

        if start_time.elapsed() > Duration::from_millis(8) {
            throttle = false;
            start_time = Instant::now();
            canvas.present();
        }
    }

    Ok(())
}
