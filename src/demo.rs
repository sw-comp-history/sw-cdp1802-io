use sw_cdp1802_asm::assemble;
use sw_cdp1802_emulator::{
    CpuState, ExecError, JoystickAxis, JoystickRcBoard, Memory, run_with_joystick,
};

pub const SCREEN_BASE: u16 = 0x0000;
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
pub const SCREEN_BYTES: usize = SCREEN_WIDTH * SCREEN_HEIGHT / 8;
pub const MAX_STEPS: u64 = 500;
pub const JOYSTICK_SOURCE: &str = include_str!("asm/joystick_lowmem.s");

#[derive(Clone, Debug)]
pub struct DemoMachine {
    pub memory: Memory,
    pub last_state: CpuState,
    pub x: u8,
    pub y: u8,
    pub last_steps: u64,
    pub crashed: bool,
    pub last_error: Option<String>,
}

impl Default for DemoMachine {
    fn default() -> Self {
        let mut machine = Self {
            memory: Memory::default(),
            last_state: CpuState::new(),
            x: 128,
            y: 128,
            last_steps: 0,
            crashed: false,
            last_error: None,
        };
        machine.reset();
        machine
    }
}

impl DemoMachine {
    pub fn reset(&mut self) {
        self.memory = Memory::default();
        self.last_state = CpuState::new();
        self.last_steps = 0;
        self.crashed = false;
        self.last_error = None;
        match assemble(JOYSTICK_SOURCE) {
            Ok(asm) => self.memory.load_bytes(0, &asm.bytes),
            Err(err) => {
                self.crashed = true;
                self.last_error = Some(format!("assemble error: {err:?}"));
            }
        }
    }

    pub fn run_frame(&mut self, x: u8, y: u8) {
        if self.crashed {
            return;
        }
        self.x = x;
        self.y = y;

        let mut state = CpuState::new();
        state.x = 15;
        let mut board = JoystickRcBoard::new(x, y);
        match run_with_joystick(&mut state, &mut self.memory, &mut board, MAX_STEPS) {
            Ok(steps) => {
                self.last_steps = steps;
                self.last_state = state;
                if !self.last_state.halted {
                    self.crashed = true;
                    self.last_error = Some("frame exceeded instruction budget".to_string());
                }
            }
            Err(err) => {
                self.last_state = state;
                self.last_steps = self.last_state.instr_count;
                self.crashed = true;
                self.last_error = Some(format_exec_error(&err));
            }
        }
    }

    pub fn x_bucket(&self) -> u8 {
        JoystickRcBoard::new(self.x, self.y).delay_for_axis(JoystickAxis::X)
    }

    pub fn y_bucket(&self) -> u8 {
        JoystickRcBoard::new(self.x, self.y).delay_for_axis(JoystickAxis::Y)
    }

    pub fn screen_bytes(&self) -> Vec<u8> {
        self.memory.read_range(SCREEN_BASE, SCREEN_BYTES)
    }
}

fn format_exec_error(err: &ExecError) -> String {
    match err {
        ExecError::Halted => "CPU was already halted".to_string(),
        ExecError::Decode(inner) => format!("decode error: {inner:?}"),
    }
}
