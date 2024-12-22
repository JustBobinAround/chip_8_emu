use std::collections::VecDeque;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAME_SIZE: usize = 4096;
const V_REGISTER_COUNT: usize = 16;
const NUM_KEYS: usize = 16;
const START_ADDR: u16 = 0x200;

pub struct Emu {
    pc: u16,
    ram:[u8; RAME_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    vregs: [u8; V_REGISTER_COUNT],
    ireg: u16,
    stack: VecDeque<u16>,
    keys: [bool; NUM_KEYS],
    dt: u8,
    st: u8,
}

impl Emu {
    pub fn new() -> Self {
        Emu { 
            pc: START_ADDR, 
            ram: [0; RAME_SIZE], 
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT], 
            vregs: [0; V_REGISTER_COUNT], 
            ireg: 0, 
            stack: VecDeque::new(), 
            keys: [false; NUM_KEYS], 
            dt: 0, 
            st: 0 
        }
    }
}
