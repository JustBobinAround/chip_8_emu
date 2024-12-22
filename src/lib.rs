use std::collections::VecDeque;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAME_SIZE: usize = 4096;
const V_REGISTER_COUNT: usize = 16;
const NUM_KEYS: usize = 16;
const START_ADDR: u16 = 0x200;

const FONTSET_SIZE: usize = 80;

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, //0
    0x20, 0x60, 0x20, 0x20, 0x70, //1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, //2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, //3
    0x90, 0x90, 0xF0, 0x10, 0x10, //4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, //5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, //6
    0xF0, 0x10, 0x20, 0x40, 0x40, //7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, //8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, //9
    0xF0, 0x90, 0xF0, 0x90, 0x90, //A
    0xF0, 0x90, 0xE0, 0x90, 0xE0, //B
    0xF0, 0x80, 0x80, 0x80, 0xF0, //C
    0xF0, 0x90, 0x90, 0x90, 0xE0, //D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, //E
    0xF0, 0x80, 0xF0, 0x80, 0x80, //F
];

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
        let mut emu = Emu { 
            pc: START_ADDR, 
            ram: [0; RAME_SIZE], 
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT], 
            vregs: [0; V_REGISTER_COUNT], 
            ireg: 0, 
            stack: VecDeque::new(), 
            keys: [false; NUM_KEYS], 
            dt: 0, 
            st: 0 
        };

        emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        emu
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR; 
        self.ram = [0; RAME_SIZE]; 
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT]; 
        self.vregs = [0; V_REGISTER_COUNT]; 
        self.ireg = 0; 
        self.stack = VecDeque::new(); 
        self.keys = [false; NUM_KEYS]; 
        self.dt = 0; 
        self.st = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn tick(&mut self) {
        let op = self.fetch_op();
    }

    fn execute(&mut self, op: u16) {
        let d1 = (op & 0xF000) >> 12;
        let d2 = (op & 0x0F00) >> 8;
        let d3 = (op & 0x00F0) >> 4;
        let d4 = op & 0x000F;

        match (d1,d2,d3,d4) {
            (0, 0, 0, 0) => return,
            (0, 0, 0xE, 0) => self.clear_screen(),
            (0, 0, 0xE, 0xE) => self.return_from_subroutine(),
            (1, _, _, _) => self.jump_to(op),
            (2, _, _, _) => self.call_subroutine(op),
            (3, _, _, _) => self.skip_next_if_vx_eq_nn(op, d2),
            (4, _, _, _) => self.skip_next_if_vx_neq_nn(op, d2),
            (5, _, _, _) => self.skip_next_if_vx_eq_vy(d2, d3),
            (6, _, _, _) => self.set_vx_as_nn(op, d2),
            (7, _, _, _) => self.inc_vx_with_nn(op, d2),
            (8, _, _, 0) => self.set_vx_as_vy(op, d2),
            (8, _, _, 1) => self.vx_binor_vy(d2, d3),
            (8, _, _, 2) => self.vx_binand_vy(d2, d3),
            (8, _, _, 3) => self.vx_binxor_vy(d2, d3),

            (_,_,_,_) => unimplemented!("rip chip-8, found unimplemented op code: {}", op)
        }
    }

    fn vx_binor_vy(&mut self, d2: u16, d3: u16) {
        let x = d2 as usize;
        let y = d3 as usize;
        self.vregs[x] |= self.vregs[y];
    }

    fn vx_binand_vy(&mut self, d2: u16, d3: u16) {
        let x = d2 as usize;
        let y = d3 as usize;
        self.vregs[x] &= self.vregs[y];
    }

    fn vx_binxor_vy(&mut self, d2: u16, d3: u16) {
        let x = d2 as usize;
        let y = d3 as usize;
        self.vregs[x] ^= self.vregs[y];
    }

    fn set_vx_as_vy(&mut self, d2: u16, d3: u16) {
        let x = d2 as usize;
        let y = d3 as usize;
        self.vregs[x] = self.vregs[y];
    }

    fn inc_vx_with_nn(&mut self, op: u16, d2: u16) {
        let x = d2 as usize;
        let nn = (op & 0xFF) as u8;
        self.vregs[x] += nn;
    }

    fn set_vx_as_nn(&mut self, op: u16, d2: u16) {
        let x = d2 as usize;
        let nn = (op & 0xFF) as u8;
        self.vregs[x] = nn;
    }

    fn skip_next_if_vx_eq_vy(&mut self, d2: u16, d3: u16) {
        let x = d2 as usize;
        let y = d3 as usize;
        if self.vregs[x]==self.vregs[y] {
            self.pc += 2;
        }
    }

    fn skip_next_if_vx_neq_nn(&mut self, op: u16, d2: u16) {
        let x = d2 as usize;
        let nn = (op & 0x0FF) as u8;
        if self.vregs[x]!=nn {
            self.pc += 2;
        }
    }

    fn skip_next_if_vx_eq_nn(&mut self, op: u16, d2: u16) {
        let x = d2 as usize;
        let nn = (op & 0x0FF) as u8;
        if self.vregs[x]==nn {
            self.pc += 2;
        }
    }

    fn call_subroutine(&mut self, op: u16) {
        self.push(self.pc); // call is just a returnable jump
        self.pc = op & 0xFFF; // return last 3 bytes to use as memory location
    }

    fn jump_to(&mut self, op: u16) {
        self.pc = op & 0xFFF; // return last 3 bytes to use as memory location
    }

    fn return_from_subroutine(&mut self) {
        self.pc = self.pop().expect("rip chip-8, popped stack to return from subroutine. No address was found");
    }

    fn clear_screen(&mut self) {
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT]; 
    }

    pub fn inc_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        if self.st > 0 {
            if self.st == 1 {
                // TODO: play beep
            }
            self.st -= 1;
        }
    }

    pub fn fetch_op(&mut self) -> u16 {
        let first_b = self.ram[self.pc as usize] as u16;
        let second_b = self.ram[self.pc as usize] as u16;
        self.pc += 2;
        (first_b << 8) | second_b
    }

    fn pop(&mut self) -> Option<u16> {
        self.stack.pop_front()
    }

    fn push(&mut self, val: u16) {
        self.stack.push_front(val);
    }
}
