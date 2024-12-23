use std::{collections::VecDeque, fs::File, io::Read, os::unix::process};
use rand::random;

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

macro_rules! gen_binop {
    ($name:ident, $op:tt) => {
        fn $name(&mut self, d2: u16, d3: u16) {
            let (x, y) = dd_as_xy(d2, d3);
            self.vregs[x] $op self.vregs[y];
        }
    };
}

pub struct Emu {
    pc: u16,
    pub ram:[u8; RAME_SIZE],
    pub screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    vregs: [u8; V_REGISTER_COUNT],
    pub ireg: u16,
    stack: VecDeque<u16>,
    keys: [bool; NUM_KEYS],
    dt: u8,
    st: u8,
    input_port: Option<u8>,
    pub debug: String,
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
            st: 0,
            input_port: None,
            debug: String::new()
        };

        emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);

        emu
    }

    pub fn load_rom(&mut self, dir: &String) {
        if let Ok(mut file) = File::open(dir) {
            let mut buf = Vec::new();
            let size = file.read_to_end(&mut buf).expect("oof");
            self.ram[START_ADDR as usize..size+START_ADDR as usize].copy_from_slice(&buf);
        } else {
            println!("oof2");
        }
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
        self.input_port = None;
    }

    pub fn tick(&mut self) {
        let op = self.fetch_op();
        self.execute(op);
    }

    pub fn execute(&mut self, op: u16) {
        let d1 = (op & 0xF000) >> 12;
        let d2 = (op & 0x0F00) >> 8;
        let d3 = (op & 0x00F0) >> 4;
        let d4 = op & 0x000F;
        self.debug.push_str(&format!("\n{} | {:x}, {:x}, {:x}, {:x}", op, d1, d2, d3, d4));

        //println!("{},{},{},{}",d1,d2,d3,d4);
        match (d1,d2,d3,d4) {
            (0, 0, 0, 0) => return,
            (0, 0, 0xE, 0) => self.clear_screen(),
            (0, 0, 0xE, 0xE) => self.return_from_subroutine(),
            //(0, _, _, _) => self.call_subroutine_no_stack(op),
            (1, _, _, _) => self.jump_to(op),
            (2, _, _, _) => self.call_subroutine(op),
            (3, _, _, _) => self.skip_next_if_vx_eq_nn(op, d2),
            (4, _, _, _) => self.skip_next_if_vx_neq_nn(op, d2),
            (5, _, _, 0) => self.skip_next_if_vx_eq_vy(d2, d3),
            (6, _, _, _) => self.set_vx_as_nn(op, d2),
            (7, _, _, _) => self.inc_vx_with_nn(op, d2),
            (8, _, _, 0) => self.set_vx_as_vy(op, d2),
            (8, _, _, 1) => self.vx_binor_vy(d2, d3),
            (8, _, _, 2) => self.vx_binand_vy(d2, d3),
            (8, _, _, 3) => self.vx_binxor_vy(d2, d3),
            (8, _, _, 4) => self.inc_vx_with_vy(d2, d3),
            (8, _, _, 5) => self.dec_vx_with_vy(d2, d3),
            (8, _, _, 6) => self.r_bitshift_vx(d2),
            (8, _, _, 7) => self.set_vx_to_vy_sub_vx(d2, d3),
            (8, _, _, 0xE) => self.l_bitshift_vx(d2),
            (9, _, _, 0) => self.skip_next_if_vx_neq_vy(d2, d3),
            (0xA, _, _, _) => self.set_ireg_as_nnn(op),
            (0xB, _, _, _) => self.jump_to_v0_plus_nnn(op),
            (0xC, _, _, _) => self.set_vx_as_rand_and_nn(op, d2),
            (0xD, _, _, _) => self.draw_sprite(d2, d3, d4),
            (0xE, _, 9, 0xE) => self.skip_next_if_key_pressed(d2),
            (0xE, _, 0xA, 1) => self.skip_next_if_not_key_pressed(d2),
            (0xF, _, 0, 7) => self.set_vx_as_dt(d2),
            (0xF, _, 0, 0xA) => self.wait_for_key_press(d2),
            (0xF, _, 1, 5) => self.set_dt_as_vx(d2),
            (0xF, _, 1, 8) => self.set_st_as_vx(d2),
            (0xF, _, 1, 0xE) => self.inc_ireg_by_vx(d2),
            (0xF, _, 2, 9) => self.set_ireg_to_font_addr(d2),
            (0xF, _, 3, 3) => self.set_ireg_to_bcd_of_vx(d2),
            (0xF, _, 5, 5) => self.load_v0_to_vx_into_ram(d2),
            (0xF, _, 6, 5) => self.load_v0_to_vx_into_ram(d2),
            //(0xF, _, 0xF, 0xB) => self.input_port_to_vx(d2),

            (_,_,_,_) => {
                panic!("rip chip-8,\n{}\n found unimplemented op code: {}: {:x}, {:x}, {:x}, {:x}",self.debug, op, d1 ,d2 ,d3 ,d4)
            }
        }
    }

    fn input_port_to_vx(&mut self, d2: u16) {
        let x = d2 as usize;
        if let Some(input_port) = self.input_port {
            self.vregs[x] = input_port;
        }
    }
    fn load_ram_into_v0_to_vx(&mut self, d2: u16) {
        let x = d2 as usize;
        let i = self.ireg as usize;
        for idx in 0..=x {
            if idx>V_REGISTER_COUNT {
                panic!("rip chip-8, ram to vx copy went out of bounds");
            } else {
                self.vregs[idx] = self.ram[i + idx];
            }
        }
    }

    fn load_v0_to_vx_into_ram(&mut self, d2: u16) {
        let x = d2 as usize;
        let i = self.ireg as usize;
        for idx in 0..=x {
            if idx>V_REGISTER_COUNT {
                panic!("rip chip-8, vx to ram copy went out of bounds");
            } else {
                self.ram[i + idx] = self.vregs[idx];
            }
        }
    }

    fn set_ireg_to_bcd_of_vx(&mut self, d2: u16) {
        let vx = self.vregs[d2 as usize];
        self.ram[self.ireg as usize] = grab_base_10_digit(vx, 2);
        self.ram[(self.ireg + 1) as usize] = grab_base_10_digit(vx, 1);
        self.ram[(self.ireg + 2) as usize] = grab_base_10_digit(vx, 0);
    }

    fn set_ireg_to_font_addr(&mut self, d2: u16) {
        self.ireg = self.vregs[d2 as usize] as u16 * 5;
    }

    fn inc_ireg_by_vx(&mut self, d2: u16) {
        self.ireg = self.ireg.wrapping_add(self.vregs[d2 as usize] as u16);
    }

    fn set_st_as_vx(&mut self, d2: u16) {
        self.st = self.vregs[d2 as usize];
    }

    fn set_dt_as_vx(&mut self, d2: u16) {
        self.dt = self.vregs[d2 as usize];
    }

    fn wait_for_key_press(&mut self, d2: u16) {
        let search_result = self.keys.into_iter().enumerate().find(|(_,key)| {
            *key
        });

        if let Some((idx, _)) = search_result{
            self.vregs[d2 as usize] = idx as u8;
        } else {
            self.pc -= 2;
        }
    }

    fn set_vx_as_dt(&mut self, d2: u16) {
        self.vregs[d2 as usize] = self.dt;
    }

    fn skip_next_if_not_key_pressed(&mut self, d2: u16) {
        self.skip_if(!self.keys[self.vregs[d2 as usize] as usize]);
    }

    fn skip_next_if_key_pressed(&mut self, d2: u16) {
        self.skip_if(self.keys[self.vregs[d2 as usize] as usize]);
    }

    fn draw_sprite(&mut self, d2: u16, d3: u16, d4: u16) {
        let (x, y) = dd_as_xy(d2, d3);
        let d4 = d4 as usize;
        let mut flipped = false;

        for y_line in 0..d4 {
            let pixels = self.ram[self.ireg as usize + y_line];
            for x_line in 0..8 {
                if (pixels & (0b1000_0000 >> x_line)) != 0 {
                    let x = (x + x_line) % SCREEN_WIDTH;
                    let y = (y + y_line) % SCREEN_HEIGHT;

                    let idx = x + SCREEN_WIDTH * y;

                    flipped |= self.screen[idx];
                    self.screen[idx] ^= true;
                }
            }
        }

        self.map_vreg_0xf(flipped);
    }

    fn set_vx_as_rand_and_nn(&mut self, op: u16, d2: u16) {
        let (nn, x) = op_d_as_nn_x(op, d2);
        self.vregs[x] = random::<u8>() & nn;
    }

    fn jump_to_v0_plus_nnn(&mut self, op: u16) {
        self.pc = (self.vregs[0] as u16) + (op & 0xFFF);
    }

    fn set_ireg_as_nnn(&mut self, op: u16) {
        self.ireg = op & 0xFFF;
    }

    fn skip_next_if_vx_neq_vy(&mut self, d2: u16, d3: u16) {
        let (x, y) = dd_as_xy(d2, d3);
        self.skip_if(self.vregs[x]!=self.vregs[y]);
    }

    fn l_bitshift_vx(&mut self, d2: u16) {
        let x = d2 as usize;
        self.vregs[0xF] = (self.vregs[x] >> 7) & 1; //dropped bit flag
        self.vregs[x] <<= 1;
    }

    fn set_vx_to_vy_sub_vx(&mut self, d2: u16, d3: u16) {
        let (x, y) = dd_as_xy(d2, d3);
        let (val, overflow) = self.vregs[y].overflowing_sub(self.vregs[x]);
        self.handle_overflow(x, val, !overflow); //invert flag for subtraction
    }

    fn r_bitshift_vx(&mut self, d2: u16) {
        let x = d2 as usize;
        self.vregs[0xF] = self.vregs[x] & 1; // dropped bit flag
        self.vregs[x] >>= 1;
    }

    fn dec_vx_with_vy(&mut self, d2: u16, d3: u16) {
        let (x, y) = dd_as_xy(d2, d3);
        let (val, overflow) = self.vregs[x].overflowing_sub(self.vregs[y]);
        self.handle_overflow(x, val, !overflow); //invert flag for subtraction
    }

    fn inc_vx_with_vy(&mut self, d2: u16, d3: u16) {
        let (x, y) = dd_as_xy(d2, d3);
        let (val, overflow) = self.vregs[x].overflowing_add(self.vregs[y]);
        self.handle_overflow(x, val, overflow);
    }


    fn handle_overflow(&mut self, x: usize, val: u8, overflow: bool) {
        self.map_vreg_0xf(overflow);
        self.vregs[x] = val;
    }

    fn map_vreg_0xf(&mut self, condition: bool) {
        if condition{
            self.vregs[0xF] = 1;
        } else {
            self.vregs[0xF] = 0;
        }
    }

    gen_binop!(vx_binor_vy, |=);
    gen_binop!(vx_binand_vy, &=);
    gen_binop!(vx_binxor_vy, ^=);

    fn set_vx_as_vy(&mut self, d2: u16, d3: u16) {
        let (x, y) = dd_as_xy(d2, d3);
        self.vregs[x] = self.vregs[y];
    }

    fn inc_vx_with_nn(&mut self, op: u16, d2: u16) {
        let (nn, x) = op_d_as_nn_x(op, d2);
        let (val, overflow ) = self.vregs[x].overflowing_add(nn);
        self.vregs[x]  = val;
    }

    fn set_vx_as_nn(&mut self, op: u16, d2: u16) {
        let (nn, x) = op_d_as_nn_x(op, d2);
        self.vregs[x] = nn;
    }

    fn skip_next_if_vx_eq_vy(&mut self, d2: u16, d3: u16) {
        let (x, y) = dd_as_xy(d2, d3);
        self.skip_if(self.vregs[x]==self.vregs[y]);
    }

    fn skip_next_if_vx_neq_nn(&mut self, op: u16, d2: u16) {
        let (nn, x) = op_d_as_nn_x(op, d2);
        self.skip_if(self.vregs[x]!=nn);
    }

    fn skip_next_if_vx_eq_nn(&mut self, op: u16, d2: u16) {
        let (nn, x) = op_d_as_nn_x(op, d2);
        self.skip_if(self.vregs[x]==nn);
    }

    fn skip_if(&mut self, condition: bool) {
        if condition {
            self.pc += 2;
        }
    }

    fn call_subroutine_no_stack(&mut self, op: u16) {
        self.pc = op & 0xFFF; // return last 3 bytes to use as memory location
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
        let second_b = self.ram[self.pc as usize+1] as u16;
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

fn op_d_as_nn_x(op: u16, d: u16) -> (u8, usize) {
    ((op & 0xFF) as u8, d as usize)
}

fn dd_as_xy(d2: u16, d3: u16) -> (usize, usize) {
    (d2 as usize, d3 as usize)
}

fn grab_base_10_digit(n: u8, i: u8) -> u8{
    (n / 10_u8.pow(i as u32)) % 10
}
