pub mod ais;
pub mod asm;
pub mod decode;
pub mod dynasm;
pub mod encode;

fn bit(word: u32, bit: u32) -> u32 {
    (word >> bit) & 1
}

fn bits(word: u32, high: u32, low: u32) -> u32 {
    let mask = (1 << (high - low + 1)) - 1;
    (word >> low) & mask
}
