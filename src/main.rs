extern crate rand;
extern crate minifb;

const FONT: [u8;80] = [
	0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
	0x20, 0x60, 0x20, 0x20, 0x70, // 1
	0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
	0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
	0x90, 0x90, 0xF0, 0x10, 0x10, // 4
	0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
	0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
	0xF0, 0x10, 0x20, 0x40, 0x40, // 7
	0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
	0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
	0xF0, 0x90, 0xF0, 0x90, 0x90, // A
	0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
	0xF0, 0x80, 0x80, 0x80, 0xF0, // C
	0xE0, 0x90, 0x90, 0x90, 0xE0, // D
	0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
	0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

struct Chip8 {
    reg: [u8; 16],
    mem: [u8; 4096],
    stack: [u16; 16],
    index: u16,
    pc: u16,
    sp: u8,
    delay_timer: u8,
    sound_timer: u8,
    keys: [bool; 16],
    display: [u32; 64*32],
    display_changed: bool
}

impl Chip8 {
    fn new () -> Chip8 {
        let mut c = Chip8 {
            reg: [0; 16],
            mem: [0; 4096],
            stack: [0; 16],
            index: 0,
            pc: 0x200,
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keys: [false; 16],
            display: [0; 64*32],
            display_changed: false
        };
        c.load(0x50, &FONT);
        
        return c;
    }

    fn load(&mut self, start: u16, buf: &[u8]) {
        for (i,val) in buf.iter().enumerate() {
            self.mem[i + (start as usize)] = *val;
        }
    }

    fn update_keys(&mut self, keys: &[bool; 16]) {
        self.keys = keys.clone();
    }

    fn fb(&self) -> &[u32] {
        &self.display
    }

    fn frame(&mut self, i: usize) {
        self.display_changed = false;
        for _i in 0..i {
            self.cycle();
        }
        self.decrement_timers();
    }

    fn cycle(&mut self) {
        self.pc += 2;

        match self.opcode() {
            0x00E0 => self.CLS00E0(),
            0x00EE => self.RET00EE(),
            0x1000..=0x1FFF => self.JP1nnn(),
            0x2000..=0x2FFF => self.CALL2nnn(),
            0x3000..=0x3FFF => self.SE3xkk(),
            0x4000..=0x4FFF => self.SNE4xkk(),
            0x5000..=0x5FFF => if self.opcode() & 0xF == 0 {self.SE5xy0()} else {panic!("invalid op {}", self.opcode())},
            0x6000..=0x6FFF => self.LD6xkk(),
            0x7000..=0x7FFF => self.ADD7xkk(),
            0x8000..=0x8FFF => match self.opcode() & 0xF {
                0x0 => self.LD8xy0(),
                0x1 => self.OR8xy1(),
                0x2 => self.AND8xy2(),
                0x3 => self.XOR8xy3(),
                0x4 => self.ADD8xy4(),
                0x5 => self.SUB8xy5(),
                0x6 => self.SHR8xy6(),
                0x7 => self.SUBN8xy7(),
                0xE => self.SHL8xyE(),
                _ => panic!("invalid op {}", self.opcode())
            }
            0x9000..=0x9FF0 => if self.opcode() & 0xF == 0 {self.SNE9xy0()} else {panic!("invalid op {}", self.opcode())},
            0xA000..=0xAFFF => self.LDAnnn(),
            0xB000..=0xBFFF => self.JPBnnn(),
            0xC000..=0xCFFF => self.RNDCxkk(),
            0xD000..=0xDFFF => self.DRWDxyn(),
            0xE000..=0xEFFF => match self.opcode() & 0xFF {
                0x9E => self.SKPEx9E(),
                0xA1 => self.SKNPExA1(),
                _ => panic!("invalid op {}", self.opcode())
            }
            0xF000..=0xFFFF => match self.opcode() & 0xFF {
                0x07 => self.LDFx07(),
                0x0A => self.LDFx0A(),
                0x15 => self.LDFx15(),
                0x18 => self.LDFx18(),
                0x1E => self.ADDFx1E(),
                0x29 => self.LDFx29(),
                0x33 => self.LDFx33(),
                0x55 => self.LDFx55(),
                0x65 => self.LDFx65(),
                _ => panic!("invalid op {}", self.opcode())
            }
            _ => panic!("invalid op {}", self.opcode())
        }
    }

    fn decrement_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

}

impl Chip8 {
    fn opcode(&mut self) -> u16 {
        (self.mem[(self.pc-2) as usize] as u16) << 8 | self.mem[(self.pc-1) as usize] as u16
    }

    fn push(&mut self, v: u16) {
        self.stack[self.sp as usize] = v;
        self.sp += 1;
    }
    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }

    fn CLS00E0(&mut self) {
        self.display.fill(0);
    }
    fn RET00EE(&mut self) {
        self.pc = self.pop();
    }
    fn JP1nnn(&mut self) {
        self.pc = self.opcode() & 0x0FFF;
    }
    fn CALL2nnn(&mut self) {
        self.push(self.pc);

        self.pc = self.opcode() & 0x0FFF;
    }
    fn SE3xkk(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let k = (self.opcode() & 0xFF) as u8;
        if self.reg[x as usize] == k { self.pc += 2 };
    }
    fn SNE4xkk(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let k = (self.opcode() & 0xFF) as u8;
        if self.reg[x as usize] != k { self.pc += 2 };
    }
    fn SE5xy0(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;
        if self.reg[x as usize] == self.reg[y as usize] { self.pc += 2 };
    }
    fn LD6xkk(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let k = (self.opcode() & 0xFF) as u8;
        self.reg[x as usize] = k;
    }
    fn ADD7xkk(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let k = (self.opcode() & 0xFF) as u8;
        self.reg[x as usize] = self.reg[x as usize].wrapping_add(k);
    }
    fn LD8xy0(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;
        self.reg[x as usize] = self.reg[y as usize]
    }
    fn OR8xy1(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;
        self.reg[x as usize] |= self.reg[y as usize]
    }
    fn AND8xy2(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;
        self.reg[x as usize] &= self.reg[y as usize]
    }
    fn XOR8xy3(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;
        self.reg[x as usize] ^= self.reg[y as usize]
    }
    fn ADD8xy4(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;

        let (sum, c) = self.reg[y as usize].overflowing_add(self.reg[x as usize]);
        self.reg[x as usize] = sum;
        self.reg[0xF] = if c {1} else {0};
    }
    fn SUB8xy5(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;
        
        self.reg[0xF] = if self.reg[x as usize] > self.reg[y as usize] {1} else {0};
        self.reg[x as usize] = self.reg[x as usize].wrapping_sub(self.reg[y as usize]);
    }
    fn SHR8xy6(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        
        self.reg[0xF] = self.reg[x as usize] & 1;
        self.reg[x as usize] >>= 1;
    }
    fn SUBN8xy7(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;
        
        self.reg[0xF] = if self.reg[y as usize] > self.reg[x as usize] {1} else {0};
        self.reg[x as usize] = self.reg[y as usize] - self.reg[x as usize];
    }
    fn SHL8xyE(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        
        self.reg[0xF] = (self.reg[x as usize] >> 7) & 1;
        self.reg[x as usize] <<= 1;
    }
    fn SNE9xy0(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;

        if self.reg[x as usize] != self.reg[y as usize] { self.pc += 2 };
    }
    fn LDAnnn(&mut self) {
        let addr = self.opcode() & 0x0FFF;
        self.index = addr;
    }
    fn JPBnnn(&mut self) {
        let addr = (self.opcode() & 0x0FFF) + self.reg[0] as u16;
        self.pc = addr;
    }
    fn RNDCxkk(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let k = (self.opcode() & 0xFF) as u8;
        self.reg[x as usize] = k & rand::random::<u8>();
    }
    fn DRWDxyn(&mut self) {
        self.display_changed = true;

        let x = (self.opcode() >> 8) & 0xF;
        let y = (self.opcode() >> 4) & 0xF;
        let n = self.opcode() as usize & 0xF;

        let xp = (self.reg[x as usize] % 64) as usize;
        let yp = (self.reg[y as usize] % 32) as usize;

        self.reg[0xF] = 0;

        for i in 0..n*8 {
            let spritex = i % 8;
            let spritey = i >> 3;
            let spritepx = self.mem[self.index as usize + spritey] & (0x80 >> spritex) != 0;
            let screenpx = &mut self.display[64*((yp+spritey)%32)+((xp+spritex)%64)];

            if spritepx {
                if *screenpx == 0xFFFFFF {
                    self.reg[0xF] = 1;
                }
                *screenpx ^= 0xFFFFFF;
            }
        }
    }
    fn SKPEx9E(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let key = self.keys[self.reg[x as usize] as usize];

        if key {
            self.pc += 2;
        }
    }
    fn SKNPExA1(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        let key = self.keys[self.reg[x as usize] as usize];

        if !key {
            self.pc += 2;
        }
    }
    fn LDFx07(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        
        self.reg[x as usize] = self.delay_timer;
    }
    fn LDFx0A(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        
        if let Some(n) = self.keys.iter().position(|&b| b) {
            self.reg[x as usize] = n as u8
        } else {
            self.pc -= 2;
        }
    }
    fn LDFx15(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        
        self.delay_timer = self.reg[x as usize];
    }
    fn LDFx18(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        
        self.sound_timer = self.reg[x as usize];
    }
    fn ADDFx1E(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        
        self.index += self.reg[x as usize] as u16;
    }
    fn LDFx29(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;
        
        self.index = 0x50 + 5*(self.reg[x as usize] as u16);
    }
    fn LDFx33(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;

        let v = self.reg[x as usize];
        self.mem[self.index as usize] = (v/100) % 10;
        self.mem[(self.index+1) as usize] = (v/10) % 10;
        self.mem[(self.index+2) as usize] = v % 10;
    }
    fn LDFx55(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;

        for i in 0..=x {
            self.mem[(i + self.index) as usize] = self.reg[i as usize];
        }
    }
    fn LDFx65(&mut self) {
        let x = (self.opcode() >> 8) & 0xF;

        for i in 0..=x {
            self.reg[i as usize] = self.mem[(self.index + i) as usize]
        }
    }
}
fn main() {
    use minifb::{Key, Window, WindowOptions};

    let mut vm = Chip8::new();

    let args: Vec<String> = std::env::args().collect();
    let f = std::fs::read(&args[1]).unwrap();
    let frameskip = if args.len() > 2 { args[2].parse::<usize>().unwrap() } else { 16 };
    
    vm.load(0x200, &f);

    let mut window = Window::new(
        "Chip8",
        256,
        128,
        WindowOptions {
            borderless: false,
            title: true,
            resize: true,
            scale: minifb::Scale::X4,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .unwrap();

    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    while window.is_open() {
        vm.update_keys(&[
            window.is_key_down(Key::X),
            window.is_key_down(Key::Key1),
            window.is_key_down(Key::Key2),
            window.is_key_down(Key::Key3),
            window.is_key_down(Key::Q),
            window.is_key_down(Key::W),
            window.is_key_down(Key::E),
            window.is_key_down(Key::A),
            window.is_key_down(Key::S),
            window.is_key_down(Key::D),
            window.is_key_down(Key::Z),
            window.is_key_down(Key::C),
            window.is_key_down(Key::Key4),
            window.is_key_down(Key::R),
            window.is_key_down(Key::F),
            window.is_key_down(Key::V)
        ]);

        vm.frame(frameskip);

        if vm.display_changed {
            window
            .update_with_buffer(vm.fb(), 64, 32)
            .unwrap();
        } else {
            window.update();
        }
        
    }
}