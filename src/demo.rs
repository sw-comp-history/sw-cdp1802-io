use sw_cdp1802_asm::{assemble, assemble_listing};
use sw_cdp1802_emulator::{BoardIo, CpuState, JoystickAxis, JoystickRcBoard, Memory};
use sw_cdp1802_isa::ExternalFlag;
use sw_cdp1802_isa::Instruction;

pub const SCREEN_BASE: u16 = 0x0000;
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
pub const SCREEN_BYTES: usize = SCREEN_WIDTH * SCREEN_HEIGHT / 8;
pub const MAX_STEPS: u64 = 80;
pub const JOYSTICK_SOURCE: &str = include_str!("asm/joystick_lowmem.s");

const PORT_CLEAR_VIDEO: u8 = 1;
const PORT_X_PULSE: u8 = 2;
const PORT_Y_PULSE: u8 = 3;
const PORT_DRAW_BALL: u8 = 4;

#[derive(Clone, Debug)]
pub struct DemoMachine {
    pub memory: Memory,
    pub last_state: CpuState,
    pub x: u8,
    pub y: u8,
    pub last_steps: u64,
    pub crashed: bool,
    pub last_error: Option<String>,
    pub program_len: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BoardAction {
    ClearVideo,
    DrawBall,
}

#[derive(Clone, Debug)]
struct WebIoBoard {
    rc: JoystickRcBoard,
    action: Option<BoardAction>,
}

impl WebIoBoard {
    fn new(x: u8, y: u8) -> Self {
        Self {
            rc: JoystickRcBoard::new(x, y),
            action: None,
        }
    }

    fn sync_inputs_to_cpu(&self, state: &mut CpuState) {
        state.set_external_flag(ExternalFlag::Ef4, self.rc.ready());
    }

    fn output_port(&mut self, port: u8) {
        match port {
            PORT_CLEAR_VIDEO => self.action = Some(BoardAction::ClearVideo),
            PORT_X_PULSE => self.rc.output_port(PORT_X_PULSE, 0),
            PORT_Y_PULSE => self.rc.output_port(PORT_Y_PULSE, 0),
            PORT_DRAW_BALL => self.action = Some(BoardAction::DrawBall),
            _ => {}
        }
    }

    fn after_instruction(&mut self) {
        self.rc.after_instruction();
    }

    fn take_action(&mut self) -> Option<BoardAction> {
        self.action.take()
    }

    fn x_bucket(&self) -> u8 {
        self.rc.delay_for_axis(JoystickAxis::X)
    }

    fn y_bucket(&self) -> u8 {
        self.rc.delay_for_axis(JoystickAxis::Y)
    }
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
            program_len: 0,
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
            Ok(asm) => {
                self.program_len = asm.bytes.len();
                self.memory.load_bytes(0, &asm.bytes);
            }
            Err(err) => {
                self.program_len = 0;
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
        let mut board = WebIoBoard::new(x, y);
        let start = state.instr_count;

        while !state.halted && state.instr_count - start < MAX_STEPS {
            if let Err(err) = self.step_web_io(&mut state, &mut board) {
                self.last_state = state;
                self.last_steps = self.last_state.instr_count - start;
                self.crashed = true;
                self.last_error = Some(err);
                return;
            }
        }

        self.last_steps = state.instr_count - start;
        self.last_state = state;
        if !self.last_state.halted {
            self.crashed = true;
            self.last_error = Some("frame exceeded instruction budget".to_string());
        }
    }

    fn step_web_io(&mut self, state: &mut CpuState, board: &mut WebIoBoard) -> Result<(), String> {
        if state.halted {
            return Err("CPU was already halted".to_string());
        }

        board.sync_inputs_to_cpu(state);
        let pc = state.pc();
        let (insn, size) = self
            .memory
            .decode_at(pc)
            .map_err(|err| format!("decode error at 0x{pc:04x}: {err:?}"))?;
        state.advance_pc(size);
        state.instr_count += 1;

        match insn {
            Instruction::Idle => state.halted = true,
            Instruction::Branch { target } => {
                let high = state.pc() & 0xff00;
                state.set_pc(high | target as u16);
            }
            Instruction::BranchExternalFlag {
                flag,
                expected,
                target,
            } => {
                if state.external_flag(flag) == expected {
                    let high = state.pc() & 0xff00;
                    state.set_pc(high | target as u16);
                }
            }
            Instruction::Output { port } => {
                let idx = state.x & 0x0f;
                let addr = state.read_reg(idx);
                let _value = self.memory.read_byte(addr);
                board.output_port(port);
                state.write_reg(idx, addr.wrapping_add(1));
            }
            other => {
                return Err(format!(
                    "unsupported web demo instruction at 0x{pc:04x}: {other:?}"
                ));
            }
        }

        board.after_instruction();
        if let Some(action) = board.take_action() {
            match action {
                BoardAction::ClearVideo => self.clear_non_code_video(),
                BoardAction::DrawBall => self.draw_ball(board.x_bucket(), board.y_bucket()),
            }
        }
        Ok(())
    }

    fn clear_non_code_video(&mut self) {
        let start = self.program_len.min(SCREEN_BYTES);
        for offset in start..SCREEN_BYTES {
            self.memory.write_byte(SCREEN_BASE + offset as u16, 0);
        }
    }

    fn draw_ball(&mut self, x_bucket: u8, y_bucket: u8) {
        let row_offsets = [0x00, 0x40, 0x80, 0xc0];
        let col_offsets = [0x00, 0x02, 0x04, 0x06];
        let addr = row_offsets[y_bucket as usize] + col_offsets[x_bucket as usize];
        self.memory.write_byte(addr, 0x80);
    }

    pub fn x_bucket(&self) -> u8 {
        JoystickRcBoard::new(self.x, self.y).delay_for_axis(JoystickAxis::X)
    }

    pub fn y_bucket(&self) -> u8 {
        JoystickRcBoard::new(self.x, self.y).delay_for_axis(JoystickAxis::Y)
    }

    pub fn active_addr(&self) -> u16 {
        if self.last_state.halted {
            self.last_state.pc().saturating_sub(1)
        } else {
            self.last_state.pc()
        }
    }

    pub fn screen_bytes(&self) -> Vec<u8> {
        self.memory.read_range(SCREEN_BASE, SCREEN_BYTES)
    }
}

pub fn assembly_listing() -> String {
    assemble_listing(JOYSTICK_SOURCE)
        .unwrap_or_else(|err| format!("assembler listing error: {err:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn included_program_stays_under_one_quarter_of_256_bytes() {
        let machine = DemoMachine::default();

        assert!(
            machine.program_len < 64,
            "program length was {}",
            machine.program_len
        );
    }

    #[test]
    fn centered_joystick_places_ball_near_center_memory() {
        let mut machine = DemoMachine::default();

        machine.run_frame(128, 128);

        assert_eq!(machine.memory.read_byte(0x0084), 0x80);
        assert!(!machine.crashed);
    }

    #[test]
    fn included_assembly_clears_non_code_video_before_sampling() {
        let mut machine = DemoMachine::default();

        machine.run_frame(128, 255);
        assert_eq!(machine.memory.read_byte(0x00c4), 0x80);

        machine.run_frame(0, 255);
        assert_eq!(machine.memory.read_byte(0x00c4), 0x00);
        assert_eq!(machine.memory.read_byte(0x00c0), 0x80);
    }

    #[test]
    fn assembler_listing_is_generated_from_included_source() {
        let listing = assembly_listing();

        assert!(listing.contains("OUT 1"));
        assert!(listing.contains("DRAW"));
    }
}
