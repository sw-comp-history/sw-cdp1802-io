use sw_cdp1802_asm::{assemble, assemble_listing};
use sw_cdp1802_emulator::{BoardIo, CpuState, JoystickRcBoard, Memory};
use sw_cdp1802_isa::ExternalFlag;
use sw_cdp1802_isa::Instruction;

pub const SCREEN_BASE: u16 = 0x0000;
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
pub const SCREEN_BYTES: usize = SCREEN_WIDTH * SCREEN_HEIGHT / 8;
pub const MAX_STEPS_PER_FRAME: u64 = 80;
pub const MAX_STEPS_PER_RUN: u64 = 400;
pub const JOYSTICK_SOURCE: &str = include_str!("asm/joystick_lowmem.s");
pub const LOGO_SOURCE: &str = include_str!("asm/logo.s");

const PORT_CLEAR_VIDEO: u8 = 1;
const PORT_X_PULSE: u8 = 2;
const PORT_Y_PULSE: u8 = 3;
const CHANGE_RECENT_STEPS: u64 = 1;
const CHANGE_OLDER_STEPS: u64 = 10;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DemoKind {
    Joystick,
    Logo,
}

impl DemoKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Joystick => "Joystick",
            Self::Logo => "Logo",
        }
    }

    pub fn source(self) -> &'static str {
        match self {
            Self::Joystick => JOYSTICK_SOURCE,
            Self::Logo => LOGO_SOURCE,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ChangeAge {
    Recent,
    Older,
    Stable,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ControlField {
    Pc,
    Active,
    D,
    Df,
    P,
    X,
    Ef4,
    Q,
}

#[derive(Clone, Debug)]
pub struct DemoMachine {
    pub memory: Memory,
    pub visible_memory: Memory,
    pub last_state: CpuState,
    pub x: u8,
    pub y: u8,
    pub last_steps: u64,
    pub crashed: bool,
    pub last_error: Option<String>,
    pub program_len: usize,
    pub last_executed_addr: Option<u16>,
    pub completed_frames: u64,
    pub last_ball_addr: Option<u16>,
    pub kind: DemoKind,
    reg_changed_at: [Option<u64>; 16],
    control_changed_at: [Option<u64>; 8],
    board: Option<WebIoBoard>,
    running: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum BoardAction {
    ClearVideo,
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
            _ => {}
        }
    }

    fn after_instruction(&mut self) {
        self.rc.after_instruction();
    }

    fn set_position(&mut self, x: u8, y: u8) {
        self.rc.set_position(x, y);
    }

    fn take_action(&mut self) -> Option<BoardAction> {
        self.action.take()
    }
}

impl Default for DemoMachine {
    fn default() -> Self {
        let mut machine = Self {
            memory: Memory::default(),
            visible_memory: Memory::default(),
            last_state: CpuState::new(),
            x: 128,
            y: 128,
            last_steps: 0,
            crashed: false,
            last_error: None,
            program_len: 0,
            last_executed_addr: None,
            completed_frames: 0,
            last_ball_addr: None,
            kind: DemoKind::Joystick,
            reg_changed_at: [None; 16],
            control_changed_at: [None; 8],
            board: None,
            running: false,
        };
        machine.reset();
        machine
    }
}

impl DemoMachine {
    pub fn reset(&mut self) {
        self.memory = Memory::default();
        self.visible_memory = Memory::default();
        self.last_state = CpuState::new();
        self.last_steps = 0;
        self.crashed = false;
        self.last_error = None;
        self.last_executed_addr = None;
        self.completed_frames = 0;
        self.last_ball_addr = None;
        self.reg_changed_at = [None; 16];
        self.control_changed_at = [None; 8];
        self.board = None;
        self.running = false;
        match assemble(self.kind.source()) {
            Ok(asm) => {
                self.program_len = asm.bytes.len();
                self.memory.load_bytes(0, &asm.bytes);
                self.visible_memory.load_bytes(0, &asm.bytes);
            }
            Err(err) => {
                self.program_len = 0;
                self.crashed = true;
                self.last_error = Some(format!("assemble error: {err:?}"));
            }
        }
    }

    #[cfg(test)]
    pub fn run_frame(&mut self, x: u8, y: u8) {
        self.start_frame(x, y);
        let target_frame = self.completed_frames + 1;
        while self.completed_frames < target_frame && self.step_frame() {}
    }

    pub fn start_frame(&mut self, x: u8, y: u8) {
        if self.crashed {
            return;
        }
        self.x = x;
        self.y = y;
        self.last_state = CpuState::new();
        self.last_state.x = 15;
        self.last_steps = 0;
        self.last_error = None;
        self.last_executed_addr = None;
        self.board = Some(WebIoBoard::new(x, y));
        self.running = true;
    }

    pub fn switch_demo(&mut self, kind: DemoKind) {
        self.kind = kind;
        self.reset();
        if kind == DemoKind::Joystick {
            self.start_frame(self.x, self.y);
        } else {
            self.start_frame(128, 128);
        }
    }

    pub fn set_position(&mut self, x: u8, y: u8) {
        self.x = x;
        self.y = y;
        if let Some(board) = self.board.as_mut() {
            board.set_position(x, y);
        }
    }

    pub fn step_frame(&mut self) -> bool {
        if self.crashed || !self.running {
            return false;
        }

        let mut state = self.last_state.clone();
        let Some(mut board) = self.board.take() else {
            self.running = false;
            self.last_error = Some("missing I/O board state".to_string());
            return false;
        };

        let before = state.clone();
        if let Err(err) = self.step_web_io(&mut state, &mut board) {
            self.last_state = state;
            self.last_steps = self.last_state.instr_count;
            self.crashed = true;
            self.running = false;
            self.last_error = Some(err);
            return false;
        }

        self.last_steps = state.instr_count;
        self.track_changes(&before, &state);
        self.last_state = state;
        self.board = Some(board);

        if self.last_state.halted {
            self.visible_memory = self.memory.clone();
            self.running = false;
            return false;
        }
        let max_steps = match self.kind {
            DemoKind::Joystick => MAX_STEPS_PER_FRAME,
            DemoKind::Logo => MAX_STEPS_PER_RUN,
        };
        if self.last_state.instr_count >= max_steps && self.completed_frames == 0 {
            self.crashed = true;
            self.running = false;
            self.last_error = Some("frame exceeded instruction budget".to_string());
            return false;
        }

        true
    }

    pub fn running(&self) -> bool {
        self.running && !self.crashed
    }

    fn step_web_io(&mut self, state: &mut CpuState, board: &mut WebIoBoard) -> Result<(), String> {
        if state.halted {
            return Err("CPU was already halted".to_string());
        }

        board.sync_inputs_to_cpu(state);
        let pc = state.pc();
        self.last_executed_addr = Some(pc);
        let (insn, size) = self
            .memory
            .decode_at(pc)
            .map_err(|err| format!("decode error at 0x{pc:04x}: {err:?}"))?;
        state.advance_pc(size);
        state.instr_count += 1;

        match insn {
            Instruction::Idle => state.halted = true,
            Instruction::ResetQ => state.q = false,
            Instruction::SetQ => state.q = true,
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
            Instruction::Store { reg } => {
                let addr = state.read_reg(reg.index_u8());
                self.memory.write_byte(addr, state.d);
                if self.kind == DemoKind::Joystick && reg.index_u8() == 1 {
                    self.last_ball_addr = Some(addr);
                    self.visible_memory = self.memory.clone();
                    self.completed_frames += 1;
                } else if self.kind == DemoKind::Logo {
                    self.visible_memory = self.memory.clone();
                }
            }
            Instruction::PutLow { reg } => {
                let idx = reg.index_u8();
                let value = (state.read_reg(idx) & 0xff00) | state.d as u16;
                state.write_reg(idx, value);
            }
            Instruction::PutHigh { reg } => {
                let idx = reg.index_u8();
                let value = ((state.d as u16) << 8) | (state.read_reg(idx) & 0x00ff);
                state.write_reg(idx, value);
            }
            Instruction::LoadImmediate { value } => state.d = value,
            Instruction::SetX { reg } => state.x = reg.index_u8(),
            Instruction::Add => {
                let value = self.memory.read_byte(state.read_reg(state.x));
                let sum = state.d as u16 + value as u16;
                state.d = sum as u8;
                state.df = sum > 0xff;
            }
            Instruction::AddImmediate { value } => {
                let sum = state.d as u16 + value as u16;
                state.d = sum as u8;
                state.df = sum > 0xff;
            }
            Instruction::ShiftLeft => {
                state.df = state.d & 0x80 != 0;
                state.d = state.d.wrapping_shl(1);
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

    pub fn active_addr(&self) -> u16 {
        if self.last_state.halted {
            self.last_executed_addr
                .unwrap_or_else(|| self.last_state.pc().saturating_sub(1))
        } else {
            self.last_state.pc()
        }
    }

    pub fn current_addr(&self) -> Option<u16> {
        (!self.last_state.halted).then_some(self.last_state.pc())
    }

    pub fn control_age(&self, field: ControlField) -> ChangeAge {
        self.change_age(self.control_changed_at[field as usize])
    }

    pub fn register_age(&self, idx: usize) -> ChangeAge {
        self.change_age(self.reg_changed_at[idx])
    }

    fn change_age(&self, changed_at: Option<u64>) -> ChangeAge {
        let Some(changed_at) = changed_at else {
            return ChangeAge::Stable;
        };
        let age = self.last_state.instr_count.saturating_sub(changed_at);
        if age <= CHANGE_RECENT_STEPS {
            ChangeAge::Recent
        } else if age <= CHANGE_OLDER_STEPS {
            ChangeAge::Older
        } else {
            ChangeAge::Stable
        }
    }

    fn track_changes(&mut self, before: &CpuState, after: &CpuState) {
        let tick = after.instr_count;
        if before.pc() != after.pc() {
            self.control_changed_at[ControlField::Pc as usize] = Some(tick);
            self.control_changed_at[ControlField::Active as usize] = Some(tick);
        }
        if before.d != after.d {
            self.control_changed_at[ControlField::D as usize] = Some(tick);
        }
        if before.df != after.df {
            self.control_changed_at[ControlField::Df as usize] = Some(tick);
        }
        if before.p != after.p {
            self.control_changed_at[ControlField::P as usize] = Some(tick);
        }
        if before.x != after.x {
            self.control_changed_at[ControlField::X as usize] = Some(tick);
        }
        if before.ef[3] != after.ef[3] {
            self.control_changed_at[ControlField::Ef4 as usize] = Some(tick);
        }
        if before.q != after.q {
            self.control_changed_at[ControlField::Q as usize] = Some(tick);
        }
        for idx in 0..16 {
            if before.read_reg(idx as u8) != after.read_reg(idx as u8) {
                self.reg_changed_at[idx] = Some(tick);
            }
        }
    }

    pub fn screen_bytes(&self) -> Vec<u8> {
        self.visible_memory.read_range(SCREEN_BASE, SCREEN_BYTES)
    }
}

pub fn listing_for(kind: DemoKind) -> String {
    assemble_listing(kind.source())
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
        assert!(!machine.crashed, "{:?}", machine.last_error);
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
    fn joystick_position_update_changes_rc_delays_without_cpu_restart() {
        let mut machine = DemoMachine::default();

        machine.start_frame(128, 128);
        while machine.completed_frames < 1 && machine.step_frame() {}
        let steps_after_first_frame = machine.last_state.instr_count;

        machine.set_position(0, 255);
        while machine.completed_frames < 2 && machine.step_frame() {}

        assert!(machine.last_state.instr_count > steps_after_first_frame);
        assert_eq!(machine.last_ball_addr, Some(0x00c0));
        assert_eq!(machine.memory.read_byte(0x0084), 0x00);
        assert_eq!(machine.memory.read_byte(0x00c0), 0x80);
        assert_eq!(machine.screen_bytes()[0x00c0], 0x80);
    }

    #[test]
    fn assembler_listing_is_generated_from_included_source() {
        let listing = listing_for(DemoKind::Joystick);

        assert!(listing.contains("OUT 1"));
        assert!(listing.contains("STR R1"));
    }

    #[test]
    fn logo_demo_draws_pixels_and_halts() {
        let mut machine = DemoMachine::default();

        machine.switch_demo(DemoKind::Logo);
        while machine.running() {
            machine.step_frame();
        }

        assert!(!machine.crashed, "{:?}", machine.last_error);
        assert!(machine.last_state.halted);
        assert_eq!(machine.memory.read_byte(0x0000), 0x00);
        assert_eq!(machine.memory.read_byte(0x005a), 0xff);
        assert_eq!(machine.screen_bytes()[0x005a], 0xff);
    }
}
