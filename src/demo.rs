use sw_cdp1802_asm::{assemble, assemble_listing};
use sw_cdp1802_emulator::{BoardIo, CpuState, JoystickAxis, JoystickRcBoard, Memory, step_with_io};
use sw_cdp1802_isa::ExternalFlag;
use sw_cdp1802_isa::Instruction;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
pub const SCREEN_BYTES: usize = SCREEN_WIDTH * SCREEN_HEIGHT / 8;
pub const MAX_STEPS_PER_FRAME: u64 = 80;
pub const MAX_STEPS_PER_RUN: u64 = 400;
pub const MAX_STEPS_PER_CASSETTE_LOAD: u64 = 4096;
pub const SCOPE_SAMPLES: usize = 48;
pub const CASSETTE_SCOPE_SAMPLES: usize = 96;
pub const ADD_SOURCE: &str = include_str!("asm/add.s");
pub const JOYSTICK_SOURCE: &str = include_str!("asm/joystick_lowmem.s");
pub const LOGO_SOURCE: &str = include_str!("asm/logo.s");
pub const PATTERN_SOURCE: &str = include_str!("asm/pattern.s");
pub const CASSETTE_SOURCE: &str = include_str!("asm/cassette_loader.s");

const PORT_CLEAR_VIDEO: u8 = 1;
const PORT_X_PULSE: u8 = 2;
const PORT_Y_PULSE: u8 = 3;
const PORT_CASSETTE_IN: u8 = 4;
const CASSETTE_LEADER_BYTES: usize = 16;
const CASSETTE_SYNC_BYTE: u8 = 0xa5;
const CHANGE_RECENT_STEPS: u64 = 1;
const CHANGE_OLDER_STEPS: u64 = 10;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DemoKind {
    Add,
    Cassette,
    Joystick,
    Logo,
    Pattern,
}

impl DemoKind {
    pub const fn all() -> [Self; 5] {
        [
            Self::Add,
            Self::Cassette,
            Self::Joystick,
            Self::Logo,
            Self::Pattern,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Add => "Add",
            Self::Cassette => "Cassette",
            Self::Joystick => "Joystick",
            Self::Logo => "Logo",
            Self::Pattern => "Pattern",
        }
    }

    pub fn source(self) -> &'static str {
        match self {
            Self::Add => ADD_SOURCE,
            Self::Cassette => CASSETTE_SOURCE,
            Self::Joystick => JOYSTICK_SOURCE,
            Self::Logo => LOGO_SOURCE,
            Self::Pattern => PATTERN_SOURCE,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Add => {
                "Add demo: step through a tiny 1802 program that loads 0x07 into D, adds 0x05 with ADI, and halts so the D register and program counter changes are visible one instruction at a time."
            }
            Self::Cassette => {
                "Cassette demo: in 4K mode, a toggled-in loader scans a simulated tape leader until it sees sync byte 0xA5, then INP 4 copies a 256-byte logo into video memory at 0x0100 while the tape waveform is shown on the scope."
            }
            Self::Joystick => {
                "Joystick demo: each axis is a potentiometer feeding an RC network connected to output strobe and EF4 input pins. The 1802 program strobes an axis, then counts in a polling loop until the capacitor echo reaches EF4; those counts position the ball in the shared 0x0000..0x00ff memory/video page. WARNING: if you move the joystick up you will crash the ball into the running program corrupting it."
            }
            Self::Logo => {
                "Logo demo: the assembled program clears the shared 256-byte memory/video page, draws a simple rocket logo from static data, and halts while the TV monitor keeps scanning the bytes as pixels."
            }
            Self::Pattern => {
                "Pattern demo: edit a small 1802 assembly program, assemble it in the browser, and run it to write a visible byte pattern into the lower half of the 256-byte memory/video page."
            }
        }
    }

    pub fn memory_map(self) -> MemoryMap {
        match self {
            Self::Add | Self::Joystick | Self::Logo | Self::Pattern => MemoryMap::elf_256(),
            Self::Cassette => MemoryMap::expanded_4k(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MemoryMap {
    pub ram_size: usize,
    pub video_base: u16,
}

impl MemoryMap {
    pub const fn elf_256() -> Self {
        Self {
            ram_size: 256,
            video_base: 0x0000,
        }
    }

    pub const fn expanded_4k() -> Self {
        Self {
            ram_size: 4096,
            video_base: 0x0100,
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ScopeSample {
    pub tick: u64,
    pub high: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CassetteScopeSample {
    pub sample: usize,
    pub high: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct ScopePulse {
    trigger_tick: u64,
    delay_ticks: u8,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct ScopeState {
    x: Option<ScopePulse>,
    y: Option<ScopePulse>,
}

impl ScopeState {
    fn record(&mut self, axis: JoystickAxis, trigger_tick: u64, delay_ticks: u8) {
        let pulse = Some(ScopePulse {
            trigger_tick,
            delay_ticks,
        });
        match axis {
            JoystickAxis::X => self.x = pulse,
            JoystickAxis::Y => self.y = pulse,
        }
    }

    fn pulse(&self, axis: JoystickAxis) -> Option<ScopePulse> {
        match axis {
            JoystickAxis::X => self.x,
            JoystickAxis::Y => self.y,
        }
    }

    fn samples(&self, axis: JoystickAxis, _current_tick: u64) -> Vec<ScopeSample> {
        (0..SCOPE_SAMPLES)
            .map(|offset| {
                let tick = offset as u64;
                ScopeSample {
                    tick,
                    high: self.is_high_at_offset(axis, tick),
                }
            })
            .collect()
    }

    fn is_high_at_offset(&self, axis: JoystickAxis, offset: u64) -> bool {
        let Some(pulse) = self.pulse(axis) else {
            return false;
        };
        offset <= u64::from(pulse.delay_ticks)
    }
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
    pub cassette_bytes_read: usize,
    pub cassette_audio: Vec<u8>,
    pub kind: DemoKind,
    pub memory_map: MemoryMap,
    pub scope: ScopeState,
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
    pulses: Vec<(JoystickAxis, u8)>,
    cassette: Vec<u8>,
    cassette_pos: usize,
}

impl WebIoBoard {
    fn new(x: u8, y: u8, cassette: Vec<u8>) -> Self {
        Self {
            rc: JoystickRcBoard::new(x, y),
            action: None,
            pulses: Vec::new(),
            cassette,
            cassette_pos: 0,
        }
    }

    fn set_position(&mut self, x: u8, y: u8) {
        self.rc.set_position(x, y);
    }

    fn take_action(&mut self) -> Option<BoardAction> {
        self.action.take()
    }

    fn take_pulses(&mut self) -> Vec<(JoystickAxis, u8)> {
        std::mem::take(&mut self.pulses)
    }

    fn read_cassette_byte(&mut self) -> u8 {
        let value = self.cassette.get(self.cassette_pos).copied().unwrap_or(0);
        self.cassette_pos = self.cassette_pos.saturating_add(1);
        value
    }

    fn cassette_pos(&self) -> usize {
        self.cassette_pos
    }
}

impl BoardIo for WebIoBoard {
    fn sync_inputs_to_cpu(&self, state: &mut CpuState) {
        state.set_external_flag(ExternalFlag::Ef4, self.rc.ready());
    }

    fn input_port(&mut self, port: u8) -> u8 {
        match port {
            PORT_CASSETTE_IN => self.read_cassette_byte(),
            _ => 0,
        }
    }

    fn output_port(&mut self, port: u8, _value: u8) {
        match port {
            PORT_CLEAR_VIDEO => self.action = Some(BoardAction::ClearVideo),
            PORT_X_PULSE => {
                self.pulses
                    .push((JoystickAxis::X, self.rc.delay_for_axis(JoystickAxis::X)));
                self.rc.output_port(PORT_X_PULSE, 0);
            }
            PORT_Y_PULSE => {
                self.pulses
                    .push((JoystickAxis::Y, self.rc.delay_for_axis(JoystickAxis::Y)));
                self.rc.output_port(PORT_Y_PULSE, 0);
            }
            _ => {}
        }
    }

    fn sync_outputs_from_cpu(&mut self, state: &CpuState) {
        self.rc.sync_outputs_from_cpu(state);
    }

    fn after_instruction(&mut self) {
        self.rc.after_instruction();
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
            cassette_bytes_read: 0,
            cassette_audio: Vec::new(),
            kind: DemoKind::Add,
            memory_map: DemoKind::Add.memory_map(),
            scope: ScopeState::default(),
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
        self.reset_with_source(self.kind.source());
    }

    pub fn reset_with_source(&mut self, source: &str) {
        self.memory_map = self.kind.memory_map();
        self.memory = Memory::new(self.memory_map.ram_size);
        self.visible_memory = Memory::new(self.memory_map.ram_size);
        self.last_state = CpuState::new();
        self.last_steps = 0;
        self.crashed = false;
        self.last_error = None;
        self.last_executed_addr = None;
        self.completed_frames = 0;
        self.last_ball_addr = None;
        self.cassette_bytes_read = 0;
        self.cassette_audio.clear();
        self.scope = ScopeState::default();
        self.reg_changed_at = [None; 16];
        self.control_changed_at = [None; 8];
        self.board = None;
        self.running = false;
        match assemble(source) {
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
        self.board = Some(WebIoBoard::new(x, y, self.cassette_stream()));
        self.running = true;
    }

    pub fn switch_demo(&mut self, kind: DemoKind) {
        self.kind = kind;
        self.reset();
        match kind {
            DemoKind::Add => {}
            DemoKind::Joystick => self.start_frame(self.x, self.y),
            DemoKind::Logo | DemoKind::Cassette => self.start_frame(128, 128),
            DemoKind::Pattern => {}
        }
    }

    pub fn run_source(&mut self, source: &str) {
        self.reset_with_source(source);
        self.start_frame(128, 128);
    }

    pub fn set_position(&mut self, x: u8, y: u8) {
        self.x = x;
        self.y = y;
        if let Some(board) = self.board.as_mut() {
            board.set_position(x, y);
        }
    }

    pub fn step_once(&mut self) -> bool {
        if self.crashed || self.last_state.halted {
            return false;
        }
        if self.board.is_none() {
            self.board = Some(WebIoBoard::new(128, 128, self.cassette_stream()));
        }
        self.running = true;
        let stepped = self.step_frame();
        if !self.last_state.halted && !self.crashed {
            self.running = false;
        }
        stepped
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
        if let Err(err) = self.step_shared_io(&mut state, &mut board) {
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

        for (axis, delay_ticks) in self.board.as_mut().expect("board restored").take_pulses() {
            self.scope
                .record(axis, self.last_state.instr_count, delay_ticks);
        }

        if self.last_state.halted {
            self.visible_memory = self.memory.clone();
            self.running = false;
            return false;
        }
        let max_steps = match self.kind {
            DemoKind::Add => MAX_STEPS_PER_RUN,
            DemoKind::Joystick => MAX_STEPS_PER_FRAME,
            DemoKind::Logo | DemoKind::Pattern => MAX_STEPS_PER_RUN,
            DemoKind::Cassette => MAX_STEPS_PER_CASSETTE_LOAD,
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

    fn step_shared_io(
        &mut self,
        state: &mut CpuState,
        board: &mut WebIoBoard,
    ) -> Result<(), String> {
        if state.halted {
            return Err("CPU was already halted".to_string());
        }

        let pc = state.pc();
        self.last_executed_addr = Some(pc);
        let (insn, _) = self
            .memory
            .decode_at(pc)
            .map_err(|err| format!("decode error at 0x{pc:04x}: {err:?}"))?;

        let joystick_ball_store = match insn {
            Instruction::Store { reg }
                if self.kind == DemoKind::Joystick && reg.index_u8() == 1 =>
            {
                Some(state.read_reg(1))
            }
            _ => None,
        };

        step_with_io(state, &mut self.memory, Some(board))
            .map_err(|err| format!("emulator error at 0x{pc:04x}: {err:?}"))?;

        if let Some(addr) = joystick_ball_store {
            self.last_ball_addr = Some(addr);
            self.visible_memory = self.memory.clone();
            self.completed_frames += 1;
        } else if matches!(
            self.kind,
            DemoKind::Add | DemoKind::Logo | DemoKind::Pattern | DemoKind::Cassette
        ) {
            self.visible_memory = self.memory.clone();
        }

        if let Some(action) = board.take_action() {
            match action {
                BoardAction::ClearVideo => self.clear_non_code_video(),
            }
        }
        let previous_cassette_pos = self.cassette_bytes_read;
        self.cassette_bytes_read = board.cassette_pos();
        if self.kind == DemoKind::Cassette && self.cassette_bytes_read > previous_cassette_pos {
            let stream = self.cassette_stream();
            self.cassette_audio.extend(
                (previous_cassette_pos..self.cassette_bytes_read)
                    .filter_map(|idx| stream.get(idx).copied()),
            );
        }
        Ok(())
    }

    fn clear_non_code_video(&mut self) {
        let start = self.program_len.min(SCREEN_BYTES);
        for offset in start..SCREEN_BYTES {
            self.memory
                .write_byte(self.memory_map.video_base + offset as u16, 0);
        }
    }

    fn cassette_stream(&self) -> Vec<u8> {
        if self.kind != DemoKind::Cassette {
            return Vec::new();
        }
        let Ok(logo) = assemble(LOGO_SOURCE) else {
            return vec![0; SCREEN_BYTES];
        };
        let mut stream = vec![0; CASSETTE_LEADER_BYTES];
        stream.push(CASSETTE_SYNC_BYTE);
        stream.extend((0..SCREEN_BYTES).map(|offset| logo.bytes.get(offset).copied().unwrap_or(0)));
        stream
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
        self.visible_memory
            .read_range(self.memory_map.video_base, SCREEN_BYTES)
    }

    pub fn scope_samples(&self, axis: JoystickAxis) -> Vec<ScopeSample> {
        self.scope.samples(axis, self.last_state.instr_count)
    }

    pub fn cassette_scope_samples(&self) -> Vec<CassetteScopeSample> {
        cassette_waveform_samples(&self.cassette_audio)
    }
}

fn cassette_waveform_samples(bytes: &[u8]) -> Vec<CassetteScopeSample> {
    let mut samples = Vec::with_capacity(CASSETTE_SCOPE_SAMPLES);
    let start_bit = bytes
        .len()
        .saturating_mul(8)
        .saturating_sub(CASSETTE_SCOPE_SAMPLES);

    for sample in 0..CASSETTE_SCOPE_SAMPLES {
        let bit_index = start_bit + sample;
        let byte = bytes.get(bit_index / 8).copied().unwrap_or(0);
        let bit = (byte & (0x80 >> (bit_index % 8))) != 0;
        let clock = sample % 2 == 0;
        samples.push(CassetteScopeSample {
            sample,
            high: clock ^ bit,
        });
    }
    samples
}

pub fn listing_for(kind: DemoKind) -> String {
    listing_for_source(kind.source())
}

pub fn listing_for_source(source: &str) -> String {
    assemble_listing(source).unwrap_or_else(|err| format!("assembler listing error: {err:?}"))
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
    fn demo_kinds_are_listed_alphabetically() {
        let labels: Vec<_> = DemoKind::all().iter().map(|kind| kind.label()).collect();

        assert_eq!(
            labels,
            vec!["Add", "Cassette", "Joystick", "Logo", "Pattern"]
        );
    }

    #[test]
    fn default_demo_is_add() {
        let machine = DemoMachine::default();

        assert_eq!(machine.kind, DemoKind::Add);
    }

    #[test]
    fn add_demo_steps_through_addition_and_halts() {
        let mut machine = DemoMachine::default();

        assert_eq!(machine.last_state.d, 0);
        assert!(machine.step_once());
        assert_eq!(machine.last_state.d, 0x07);
        assert!(!machine.running());

        assert!(machine.step_once());
        assert_eq!(machine.last_state.d, 0x0c);
        assert!(!machine.last_state.halted);

        assert!(!machine.step_once());
        assert!(machine.last_state.halted);
    }

    #[test]
    fn every_demo_has_a_description() {
        for kind in DemoKind::all() {
            assert!(!kind.description().is_empty(), "{kind:?}");
        }
    }

    #[test]
    fn joystick_description_ends_with_warning() {
        assert!(DemoKind::Joystick.description().ends_with(
            "WARNING: if you move the joystick up you will crash the ball into the running program corrupting it."
        ));
    }

    #[test]
    fn centered_joystick_places_ball_near_center_memory() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Joystick);

        machine.run_frame(128, 128);

        assert_eq!(machine.memory.read_byte(0x0084), 0x80);
        assert!(!machine.crashed, "{:?}", machine.last_error);
    }

    #[test]
    fn included_assembly_clears_non_code_video_before_sampling() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Joystick);

        machine.run_frame(128, 255);
        assert_eq!(machine.memory.read_byte(0x00c4), 0x80);

        machine.run_frame(0, 255);
        assert_eq!(machine.memory.read_byte(0x00c4), 0x00);
        assert_eq!(machine.memory.read_byte(0x00c0), 0x80);
    }

    #[test]
    fn joystick_position_update_changes_rc_delays_without_cpu_restart() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Joystick);

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
    fn step_frame_executes_only_one_cpu_instruction() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Joystick);
        machine.start_frame(128, 128);

        assert!(machine.step_frame());

        assert_eq!(machine.last_state.instr_count, 1);
    }

    #[test]
    fn rc_scope_triggers_y_trace_from_output_pulse_and_drops_after_delay() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Joystick);
        machine.start_frame(128, 255);

        for _ in 0..MAX_STEPS_PER_FRAME {
            if machine.scope.pulse(JoystickAxis::Y).is_some() {
                break;
            }
            assert!(machine.step_frame());
        }

        let pulse = machine.scope.pulse(JoystickAxis::Y).expect("Y pulse");
        assert_eq!(pulse.delay_ticks, 3);
        assert!(machine.scope.is_high_at_offset(JoystickAxis::Y, 0));
        assert!(machine.scope.is_high_at_offset(JoystickAxis::Y, 3));
        assert!(!machine.scope.is_high_at_offset(JoystickAxis::Y, 4));
    }

    #[test]
    fn rc_scope_records_x_and_y_independently() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Joystick);
        machine.start_frame(0, 255);

        for _ in 0..MAX_STEPS_PER_FRAME {
            if machine.scope.pulse(JoystickAxis::X).is_some() {
                break;
            }
            assert!(machine.step_frame());
        }

        let x = machine.scope.pulse(JoystickAxis::X).expect("X pulse");
        let y = machine.scope.pulse(JoystickAxis::Y).expect("Y pulse");
        assert_eq!(x.delay_ticks, 0);
        assert_eq!(y.delay_ticks, 3);
        assert!(x.trigger_tick > y.trigger_tick);
        assert!(machine.scope.is_high_at_offset(JoystickAxis::X, 0));
        assert!(!machine.scope.is_high_at_offset(JoystickAxis::X, 1));
    }

    #[test]
    fn rc_scope_samples_are_triggered_sweeps_not_sliding_time_windows() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Joystick);
        machine.start_frame(0, 255);

        for _ in 0..MAX_STEPS_PER_FRAME {
            if machine.scope.pulse(JoystickAxis::X).is_some() {
                break;
            }
            assert!(machine.step_frame());
        }

        let at_trigger = machine.scope_samples(JoystickAxis::Y);
        let trigger_tick = machine.last_state.instr_count;
        for _ in 0..8 {
            assert!(machine.step_frame());
        }
        let later = machine.scope_samples(JoystickAxis::Y);

        assert!(machine.last_state.instr_count > trigger_tick);
        assert_eq!(at_trigger, later);
        assert_eq!(later[0].tick, 0);
        assert!(later[0].high);
        assert!(later[3].high);
        assert!(!later[4].high);
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

    #[test]
    fn pattern_demo_assembles_and_draws_lower_half() {
        let mut machine = DemoMachine::default();

        machine.kind = DemoKind::Pattern;
        machine.run_source(PATTERN_SOURCE);
        while machine.running() {
            machine.step_frame();
        }

        assert!(!machine.crashed, "{:?}", machine.last_error);
        assert!(machine.last_state.halted);
        assert_eq!(machine.memory.read_byte(0x0080), 0xaa);
        assert_eq!(machine.memory.read_byte(0x0088), 0xf0);
        assert_eq!(machine.memory.read_byte(0x0090), 0xff);
        assert_eq!(machine.screen_bytes()[0x0080], 0xaa);
    }

    #[test]
    fn cassette_demo_uses_4k_memory_and_video_page_at_0100() {
        let mut machine = DemoMachine::default();

        machine.switch_demo(DemoKind::Cassette);

        assert_eq!(machine.memory.size_bytes(), 4096);
        assert_eq!(machine.memory_map.video_base, 0x0100);
        assert!(machine.program_len < 0x0100);
        assert_eq!(machine.screen_bytes(), vec![0; SCREEN_BYTES]);
    }

    #[test]
    fn cassette_loader_consumes_stream_and_writes_video_through_cpu() {
        let logo = assemble(LOGO_SOURCE).unwrap();
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Cassette);

        while machine.cassette_bytes_read < CASSETTE_LEADER_BYTES && machine.running() {
            machine.step_frame();
        }
        assert_eq!(machine.screen_bytes(), vec![0; SCREEN_BYTES]);

        let target_read = CASSETTE_LEADER_BYTES + 1 + 0x5a;
        while machine.cassette_bytes_read <= target_read && machine.running() {
            machine.step_frame();
        }

        assert!(!machine.crashed, "{:?}", machine.last_error);
        assert!(machine.running());
        assert!(machine.cassette_bytes_read > 0x5a);
        assert_eq!(machine.memory.read_byte(0x0100 + 0x5a), logo.bytes[0x5a]);
        assert_eq!(machine.screen_bytes()[0x5a], logo.bytes[0x5a]);
    }

    #[test]
    fn cassette_audio_waveform_advances_with_consumed_bytes() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Cassette);

        let initial = machine.cassette_scope_samples();
        while machine.cassette_bytes_read < CASSETTE_LEADER_BYTES + 4 && machine.running() {
            machine.step_frame();
        }
        let after_reads = machine.cassette_scope_samples();

        assert_eq!(machine.cassette_audio.len(), machine.cassette_bytes_read);
        assert!(machine.cassette_bytes_read >= 4);
        assert_ne!(initial, after_reads);
        assert!(after_reads.iter().any(|sample| sample.high));
        assert!(after_reads.iter().any(|sample| !sample.high));
    }

    #[test]
    fn cassette_loader_loads_full_256_byte_logo_and_halts() {
        let mut machine = DemoMachine::default();
        machine.switch_demo(DemoKind::Cassette);
        let cassette = machine.cassette_stream();

        while machine.running() {
            machine.step_frame();
        }

        assert!(!machine.crashed, "{:?}", machine.last_error);
        assert!(machine.last_state.halted);
        assert_eq!(
            machine.cassette_bytes_read,
            CASSETTE_LEADER_BYTES + 1 + SCREEN_BYTES
        );
        assert_eq!(
            machine.screen_bytes(),
            cassette[CASSETTE_LEADER_BYTES + 1..].to_vec()
        );
        assert_ne!(machine.memory.read_range(0, machine.program_len), cassette);
    }
}
