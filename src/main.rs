use std::fs;

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

fn main() {
    let mut contents: Vec<bool> = vec![];
    for bits in fs::read("programs/ibm.ch8").expect("Could not read chip-8 program").into_iter().map(|b| u8_to_bits(b)) {
        contents.extend(bits);
    }
    println!("Contents: {:?}", contents);
 
    // expect("Could not read .ch8 file")
    // Initialize registers, pointers, and memory
    let mut pc:u16;
    let mut index:u16;
    let mut variable: [u8; 16] = [0; 16];
    let mut stack: Vec<u8>;
    let mut memory: [u8; 4096] = [0; 4096];

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

    // TODO: Load instructions into memory
    println!("first opcode: {:?}", bits_to_hex(&contents[0..16]));
    println!("second opcode: {:?}", bits_to_hex(&contents[16..32]));
}