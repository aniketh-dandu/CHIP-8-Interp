use std::{fs};

extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point};
use std::time::Duration;

fn u8_to_bits(num:u8) -> [bool; 8] {
    let mut bitarray: [bool;8] = [false; 8];
    for i in 0..8 {
        bitarray[7-i] = ((num >> i) & 1) == 1;
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

fn bits_to_hex(num: &[bool]) -> String {
    let num_len = num.len();
    let num_nibbles = num_len / 4;
    let remainder = num_len % 4;
    let mut ret_str = String::with_capacity(num_len);
    for i in 0..num_nibbles {
        let hex_str = bits_to_num(&num[(4*i)..(4*(i+1))]);
        ret_str.push_str(format!("{:X}", hex_str).as_str());
    }
    if remainder != 0 {
        let remain_num = bits_to_num(&num[4*num_nibbles..]);
        ret_str.push_str(format!("{:X}", remain_num).as_str());

    }
    return ret_str;
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
    let contents: Vec<u8> = fs::read("programs/emulogo.ch8").expect("Could not read chip 8 program");
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

        let opcode: String = format!("{}{}",bits_to_hex(&u8_to_bits(memory[pc])),bits_to_hex(&u8_to_bits(memory[pc+1])));
        let start_nib: char = opcode.chars().next().unwrap();
        match start_nib {
            '0' => {
                match opcode.as_str() {
                    "00E0" => {
                        canvas.set_draw_color(Color::RGB(0, 0, 0));
                        canvas.clear();
                        disp_mem = [false; 2048];
                    },
                    "00EE" => {
                        pc = stack.pop().unwrap();
                    },
                    _ => {
                        println!("Instruction not found!");
                        println!("{}",opcode);
                    }
                }
            },
            '1' => {
                // 1NNN
                // Jump to addr (set program counter)
                pc  = usize::from_str_radix(&opcode[1..], 16).unwrap();
            },
            '2' => {
                // 2NNN
                stack.push(pc);
                pc = usize::from_str_radix(&opcode[1..], 16).unwrap();
            },
            '3' => {
                // 3XNN
                if registers[opcode.chars().nth(1).unwrap().to_digit(16).unwrap() as usize] == u8::from_str_radix(&opcode[2..], 16).unwrap() {
                    pc += 2;
                }
            },
            '4' => {
                // 4XNN
                if registers[opcode.chars().nth(1).unwrap().to_digit(16).unwrap() as usize] != u8::from_str_radix(&opcode[2..], 16).unwrap() {
                    pc += 2;
                }
            },
            '5' => {
                // 5XY0
                if registers[opcode.chars().nth(1).unwrap().to_digit(16).unwrap() as usize] == registers[opcode.chars().nth(1).unwrap().to_digit(16).unwrap() as usize] {
                    pc += 2;
                }
            },
            '6' => {
                // 6XNN
                // Set register X to NN
                registers[opcode.chars().nth(1).unwrap().to_digit(16).unwrap() as usize] = u8::from_str_radix(&opcode[2..], 16).unwrap();
            },
            '7' => {
                // 7XNN
                // Add NN to register X
                registers[opcode.chars().nth(1).unwrap().to_digit(16).unwrap() as usize] += u8::from_str_radix(&opcode[2..], 16).unwrap();
            },
            'A' => {
                // ANNN
                // Set index to NNN
                index = usize::from_str_radix(&opcode[1..], 16).unwrap(); 
            },
            'B' => {
                pc = usize::from_str_radix(&opcode[1..], 16).unwrap() + registers[0] as usize;
            },
            // TODO: Fix timing to remove jittering
            'D' => {
                // DXYN
                let x: u8 = registers[opcode.chars().nth(1).unwrap().to_digit(16).unwrap() as usize];
                let y: u8 = registers[opcode.chars().nth(2).unwrap().to_digit(16).unwrap() as usize];
                let height: u16 = opcode.chars().nth(3).unwrap().to_digit(16).unwrap() as u16;
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
                            canvas.draw_point(Point::new(col as i32, row as i32)).unwrap();
                        }
                    }
                }
            },
            'F' => {
                if &opcode[2..] == "1E" {
                    index += registers[opcode.chars().nth(1).unwrap().to_digit(16).unwrap() as usize] as usize;
                }
            },
            _ => {
                println!("{}", opcode);
                break;
            },
        }
        pc += 2;

        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 700));
        // The rest of the game loop goes here...
    }

    Ok(())
}