use sw_cdp1802_emulator::JoystickAxis;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{Event, HtmlSelectElement, HtmlTextAreaElement, InputEvent, MouseEvent};
use yew::prelude::*;

use crate::demo::{
    ChangeAge, ControlField, DemoKind, DemoMachine, PATTERN_SOURCE, SCREEN_HEIGHT, SCREEN_WIDTH,
    listing_for, listing_for_source,
};

const PAD: f64 = 16.0;
const JOYSTICK_SIZE: f64 = 170.0;
const HANDLE_RADIUS: f64 = 9.0;
const CELL_WIDTH: usize = 4;
const CELL_HEIGHT: usize = 8;
const STEP_DELAY_MS: i32 = 16;
const SCOPE_WIDTH: f64 = 190.0;
const SCOPE_HEIGHT: f64 = 92.0;

pub struct App {
    machine: DemoMachine,
    dragging: bool,
    listing: String,
    source: String,
    tick_pending: bool,
    target_x: u8,
    target_y: u8,
}

pub enum Msg {
    SelectDemo(Event),
    SourceChanged(InputEvent),
    AssembleSource,
    RunSource,
    StartDrag(MouseEvent),
    Drag(MouseEvent),
    StopDrag,
    Reset,
    Tick,
    StepAdd,
    RunCassette,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let machine = DemoMachine::default();
        let listing = listing_for(machine.kind);
        let mut app = Self {
            machine,
            dragging: false,
            listing,
            source: PATTERN_SOURCE.to_string(),
            tick_pending: false,
            target_x: 128,
            target_y: 128,
        };
        if let Some(kind) = initial_demo_from_query() {
            app.machine.switch_demo(kind);
            app.listing = listing_for(kind);
        }
        app
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.schedule_tick(ctx);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::SelectDemo(event) => {
                let select = event.target_unchecked_into::<HtmlSelectElement>();
                let kind = demo_from_value(&select.value()).unwrap_or(DemoKind::Add);
                self.target_x = 128;
                self.target_y = 128;
                if kind == DemoKind::Pattern {
                    self.source = PATTERN_SOURCE.to_string();
                    self.machine.kind = kind;
                    self.machine.reset_with_source(&self.source);
                    self.listing = listing_for_source(&self.source);
                } else if matches!(kind, DemoKind::Add | DemoKind::Cassette) {
                    self.machine.switch_demo(kind);
                    self.listing = listing_for(kind);
                } else {
                    self.machine.switch_demo(kind);
                    self.listing = listing_for(kind);
                    self.schedule_tick(ctx);
                }
                true
            }
            Msg::SourceChanged(event) => {
                let textarea = event.target_unchecked_into::<HtmlTextAreaElement>();
                self.source = textarea.value();
                true
            }
            Msg::AssembleSource => {
                if self.machine.kind == DemoKind::Pattern {
                    self.machine.reset_with_source(&self.source);
                    self.listing = listing_for_source(&self.source);
                }
                true
            }
            Msg::RunSource => {
                if self.machine.kind == DemoKind::Pattern {
                    self.machine.run_source(&self.source);
                    self.listing = listing_for_source(&self.source);
                    self.schedule_tick(ctx);
                }
                true
            }
            Msg::StartDrag(event) => {
                if self.machine.kind != DemoKind::Joystick {
                    return false;
                }
                self.dragging = true;
                self.update_from_event(&event);
                self.schedule_tick(ctx);
                true
            }
            Msg::Drag(event) => {
                if self.dragging && self.machine.kind == DemoKind::Joystick {
                    self.update_from_event(&event);
                    self.schedule_tick(ctx);
                    true
                } else {
                    false
                }
            }
            Msg::StopDrag => {
                self.dragging = false;
                true
            }
            Msg::Reset => {
                self.target_x = 128;
                self.target_y = 128;
                if self.machine.kind == DemoKind::Pattern {
                    self.source = PATTERN_SOURCE.to_string();
                    self.machine.reset_with_source(&self.source);
                    self.listing = listing_for_source(&self.source);
                } else if matches!(self.machine.kind, DemoKind::Add | DemoKind::Cassette) {
                    self.machine.reset();
                    self.listing = listing_for(self.machine.kind);
                } else {
                    self.machine.reset();
                    self.machine.start_frame(128, 128);
                    self.schedule_tick(ctx);
                }
                true
            }
            Msg::Tick => {
                self.tick_pending = false;
                self.machine.step_frame();
                self.schedule_tick(ctx);
                true
            }
            Msg::StepAdd => {
                if self.machine.kind == DemoKind::Add {
                    self.machine.step_once();
                    self.listing = listing_for(DemoKind::Add);
                    true
                } else {
                    false
                }
            }
            Msg::RunCassette => {
                if self.machine.kind == DemoKind::Cassette {
                    self.machine.start_frame(128, 128);
                    self.schedule_tick(ctx);
                    true
                } else {
                    false
                }
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <main class="app-shell">
                <header class="topbar">
                    <div>
                        <p class="eyebrow">{"RCA CDP1802 / COSMAC ELF-II I/O"}</p>
                        <h1>{"CDP1802 Emulator and Assembler Demos"}</h1>
                        <p class="demo-subtitle">{ format!("{} live demo", self.machine.kind.label()) }</p>
                    </div>
                    <div class="status-strip">
                        <span class={classes!("status-dot", self.machine.crashed.then_some("bad"))}></span>
                        <span>{ self.status_label() }</span>
                    </div>
                </header>

                <section class="demo-grid">
                    <div class="panel controls-panel">
                        <div class="panel-head">
                            <h2>{"Demo"}</h2>
                            <button type="button" onclick={link.callback(|_| Msg::Reset)}>{"Reset"}</button>
                        </div>
                        { self.view_demo_picker(link) }
                        { self.view_demo_controls(link) }
                        { self.view_registers() }
                    </div>

                    <div class="panel monitor-panel">
                        <div class="panel-head">
                            <h2>{"TV monitor"}</h2>
                            <span>{"64 x 32, 256-byte page"}</span>
                        </div>
                        { self.view_monitor() }
                        <p class="note">{ self.monitor_note() }</p>
                    </div>

                    <div class="panel listing-panel">
                        <div class="panel-head">
                            <h2>{"Assembler listing"}</h2>
                            <span>{ format!("{}: {} bytes", self.machine.kind.label(), self.machine.program_len) }</span>
                        </div>
                        { self.view_listing() }
                    </div>
                </section>
                { self.view_footer() }
            </main>
        }
    }
}

impl App {
    fn update_from_event(&mut self, event: &MouseEvent) {
        let span = JOYSTICK_SIZE - PAD * 2.0;
        self.target_x =
            ((event.offset_x() as f64 - PAD).clamp(0.0, span) / span * 255.0).round() as u8;
        self.target_y =
            ((event.offset_y() as f64 - PAD).clamp(0.0, span) / span * 255.0).round() as u8;
        self.machine.set_position(self.target_x, self.target_y);
    }

    fn schedule_tick(&mut self, ctx: &Context<Self>) {
        if self.tick_pending || !self.machine.running() {
            return;
        }

        // Run one 1802 instruction per browser callback so long demos yield
        // back to the UI thread between emulator steps.
        self.tick_pending = true;
        let link = ctx.link().clone();
        let callback = Closure::once(move || link.send_message(Msg::Tick));
        web_sys::window()
            .expect("browser window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                STEP_DELAY_MS,
            )
            .expect("schedule CPU step");
        callback.forget();
    }

    fn view_demo_picker(&self, link: &html::Scope<Self>) -> Html {
        let options = DemoKind::all().into_iter().map(|kind| {
            html! {
                <option value={demo_value(kind)} selected={self.machine.kind == kind}>{ kind.label() }</option>
            }
        });
        html! {
            <select class="demo-select" onchange={link.callback(Msg::SelectDemo)}>
                { for options }
            </select>
        }
    }

    fn view_demo_controls(&self, link: &html::Scope<Self>) -> Html {
        match self.machine.kind {
            DemoKind::Add => html! {
                <div class="logo-demo-note">
                    <span>{"LDI 0x07; ADI 0x05; IDL"}</span>
                    <button
                        type="button"
                        onclick={link.callback(|_| Msg::StepAdd)}
                        disabled={self.machine.last_state.halted || self.machine.crashed}
                    >
                        {"Step"}
                    </button>
                </div>
            },
            DemoKind::Joystick => html! {
                <>
                    { self.view_joystick(link) }
                    <div class="readouts">
                        <span>{ format!("X {:03}", self.target_x) }</span>
                        <span>{ format!("Y {:03}", self.target_y) }</span>
                        <span>{ format!("bucket {},{}", axis_bucket(self.target_x), axis_bucket(self.target_y)) }</span>
                    </div>
                    { self.view_scope() }
                </>
            },
            DemoKind::Logo => html! {
                <div class="logo-demo-note">
                    <span>{"static video data"}</span>
                    <span>{"single IDL instruction"}</span>
                    <span>{"video page 0x0000..0x00ff"}</span>
                </div>
            },
            DemoKind::Pattern => self.view_source_editor(link),
            DemoKind::Cassette => html! {
                <>
                    <div class="logo-demo-note">
                        <span>{"4K loader at 0x0000"}</span>
                        <span>{"cassette stream fills video page 0x0100"}</span>
                        <span>{ format!("{} cassette bytes read", self.machine.cassette_bytes_read) }</span>
                        <button
                            type="button"
                            onclick={link.callback(|_| Msg::RunCassette)}
                            disabled={self.machine.running() || self.machine.last_state.halted || self.machine.crashed}
                        >
                            {"Run"}
                        </button>
                    </div>
                    { self.view_cassette_scope() }
                </>
            },
        }
    }

    fn view_source_editor(&self, link: &html::Scope<Self>) -> Html {
        html! {
            <div class="source-editor">
                <div class="source-actions">
                    <button type="button" onclick={link.callback(|_| Msg::AssembleSource)} disabled={self.machine.running()}>{"Assemble"}</button>
                    <button type="button" onclick={link.callback(|_| Msg::RunSource)} disabled={self.machine.running()}>{"Run"}</button>
                </div>
                <textarea
                    class="asm-source"
                    spellcheck="false"
                    value={self.source.clone()}
                    oninput={link.callback(Msg::SourceChanged)}
                />
            </div>
        }
    }

    fn status_label(&self) -> &'static str {
        if self.machine.crashed {
            "fault"
        } else if self.machine.running() {
            "running"
        } else if self.machine.last_state.halted {
            "halted"
        } else {
            "ready"
        }
    }

    fn view_footer(&self) -> Html {
        html! {
            <footer class="site-footer">
                <span>{"MIT License"}</span>
                <span class="footer-sep">{"|"}</span>
                <span>{"(c) 2026 Michael A. Wright"}</span>
                <span class="footer-sep">{"|"}</span>
                <a href="https://github.com/sw-comp-history/sw-cdp1802-io" target="_blank">{"Web demo source"}</a>
                <span class="footer-sep">{"|"}</span>
                <a href="https://github.com/sw-comp-history/sw-cdp1802-emulator" target="_blank">{"Emulator"}</a>
                <span class="footer-sep">{"|"}</span>
                <a href="https://github.com/sw-comp-history/sw-cdp1802-asm" target="_blank">{"Assembler"}</a>
                <span class="footer-sep">{"|"}</span>
                <a href="https://github.com/sw-comp-history/sw-cdp1802-isa" target="_blank">{"ISA"}</a>
                <span class="footer-sep">{"|"}</span>
                <a href="https://software-wrighter-lab.github.io/" target="_blank">{"Blog"}</a>
                <span class="footer-sep">{"|"}</span>
                <a href="https://www.youtube.com/@SoftwareWrighter" target="_blank">{"YouTube"}</a>
                <span class="footer-sep">{"|"}</span>
                <span>{ format!(
                    "{} | {} | {}",
                    env!("BUILD_HOST"),
                    env!("BUILD_SHA"),
                    env!("BUILD_TIMESTAMP"),
                ) }</span>
            </footer>
        }
    }

    fn view_joystick(&self, link: &html::Scope<Self>) -> Html {
        let span = JOYSTICK_SIZE - PAD * 2.0;
        let cx = PAD + (self.target_x as f64 / 255.0) * span;
        let cy = PAD + (self.target_y as f64 / 255.0) * span;
        html! {
            <svg class="joystick" viewBox={format!("0 0 {JOYSTICK_SIZE} {JOYSTICK_SIZE}")}
                onmousedown={link.callback(Msg::StartDrag)}
                onmousemove={link.callback(Msg::Drag)}
                onmouseup={link.callback(|_| Msg::StopDrag)}
                onmouseleave={link.callback(|_| Msg::StopDrag)}>
                <rect x={PAD.to_string()} y={PAD.to_string()} width={span.to_string()} height={span.to_string()} />
                <line x1={(JOYSTICK_SIZE / 2.0).to_string()} y1={PAD.to_string()} x2={(JOYSTICK_SIZE / 2.0).to_string()} y2={(JOYSTICK_SIZE - PAD).to_string()} />
                <line x1={PAD.to_string()} y1={(JOYSTICK_SIZE / 2.0).to_string()} x2={(JOYSTICK_SIZE - PAD).to_string()} y2={(JOYSTICK_SIZE / 2.0).to_string()} />
                <circle class="handle" cx={cx.to_string()} cy={cy.to_string()} r={HANDLE_RADIUS.to_string()} />
            </svg>
        }
    }

    fn monitor_note(&self) -> String {
        self.machine.kind.description().to_string()
    }

    fn view_scope(&self) -> Html {
        let x_samples = self.machine.scope_samples(JoystickAxis::X);
        let y_samples = self.machine.scope_samples(JoystickAxis::Y);
        let x_points = scope_points(&x_samples, 16.0, 34.0, SCOPE_WIDTH);
        let y_points = scope_points(&y_samples, 56.0, 74.0, SCOPE_WIDTH);
        html! {
            <section class="scope-panel" aria-label="RC oscilloscope">
                <div class="scope-head">
                    <span>{"RC scope"}</span>
                    <span>{ format!("tick {}", self.machine.last_state.instr_count) }</span>
                </div>
                <svg class="scope" viewBox={format!("0 0 {SCOPE_WIDTH} {SCOPE_HEIGHT}")}>
                    <line class="scope-grid-line" x1="0" y1="34" x2={SCOPE_WIDTH.to_string()} y2="34" />
                    <line class="scope-grid-line" x1="0" y1="74" x2={SCOPE_WIDTH.to_string()} y2="74" />
                    <text x="5" y="14">{"X"}</text>
                    <text x="5" y="54">{"Y"}</text>
                    <polyline class="scope-trace x-trace" points={x_points} />
                    <polyline class="scope-trace y-trace" points={y_points} />
                </svg>
            </section>
        }
    }

    fn view_cassette_scope(&self) -> Html {
        let samples = self.machine.cassette_scope_samples();
        let points = cassette_scope_points(&samples, 18.0, 58.0, SCOPE_WIDTH);
        html! {
            <section class="scope-panel cassette-scope-panel" aria-label="Cassette audio waveform">
                <div class="scope-head">
                    <span>{"Tape audio"}</span>
                    <span>{ format!("byte {}", self.machine.cassette_bytes_read) }</span>
                </div>
                <svg class="scope" viewBox={format!("0 0 {SCOPE_WIDTH} 72")}>
                    <line class="scope-grid-line" x1="0" y1="58" x2={SCOPE_WIDTH.to_string()} y2="58" />
                    <polyline class="scope-trace tape-trace" points={points} />
                </svg>
            </section>
        }
    }

    fn view_registers(&self) -> Html {
        let regs = (0..16).map(|idx| {
            html! {
                <>
                    <dt>{ format!("R{:X}", idx) }</dt>
                    <dd class={change_class(self.machine.register_age(idx as usize))}>{ format!("0x{:04x}", self.machine.last_state.read_reg(idx)) }</dd>
                </>
            }
        });
        html! {
            <section class="register-panel" aria-label="CPU registers">
                <div class="register-summary">
                    <span>{ format!("step {}", self.machine.last_steps) }</span>
                    <span>{ format!("frame {}", self.machine.completed_frames) }</span>
                    <span>{ if self.machine.running() { "stepping" } else if self.machine.last_state.halted { "halted" } else { "ready" } }</span>
                    <span>{ self.machine.last_error.as_deref().unwrap_or("ok") }</span>
                </div>
                <dl class="control-registers">
                    <dt>{"PC"}</dt><dd class={change_class(self.machine.control_age(ControlField::Pc))}>{ format!("0x{:04x}", self.machine.last_state.pc()) }</dd>
                    <dt>{"active"}</dt><dd class={change_class(self.machine.control_age(ControlField::Active))}>{ format!("0x{:04x}", self.machine.active_addr()) }</dd>
                    <dt>{"D"}</dt><dd class={change_class(self.machine.control_age(ControlField::D))}>{ format!("0x{:02x}", self.machine.last_state.d) }</dd>
                    <dt>{"DF"}</dt><dd class={change_class(self.machine.control_age(ControlField::Df))}>{ self.machine.last_state.df.to_string() }</dd>
                    <dt>{"P"}</dt><dd class={change_class(self.machine.control_age(ControlField::P))}>{ format!("R{:X}", self.machine.last_state.p) }</dd>
                    <dt>{"X"}</dt><dd class={change_class(self.machine.control_age(ControlField::X))}>{ format!("R{:X}", self.machine.last_state.x) }</dd>
                    <dt>{"EF4"}</dt><dd class={change_class(self.machine.control_age(ControlField::Ef4))}>{ self.machine.last_state.ef[3].to_string() }</dd>
                    <dt>{"Q"}</dt><dd class={change_class(self.machine.control_age(ControlField::Q))}>{ self.machine.last_state.q.to_string() }</dd>
                </dl>
                <dl class="register-grid">
                    { for regs }
                </dl>
            </section>
        }
    }

    fn view_monitor(&self) -> Html {
        let bytes = self.machine.screen_bytes();
        let program_len = self.machine.program_len;
        let ball_addr = self.machine.last_ball_addr.map(usize::from);
        let video_base = usize::from(self.machine.memory_map.video_base);
        let pixels = (0..SCREEN_HEIGHT).flat_map(|row| {
            let bytes = &bytes;
            (0..SCREEN_WIDTH).map(move |col| {
                let byte_index = row * (SCREEN_WIDTH / 8) + col / 8;
                let mem_addr = video_base + byte_index;
                let byte = bytes[byte_index];
                let set = (byte & (0x80 >> (col % 8))) != 0;
                let ball = set && ball_addr == Some(mem_addr);
                let class = classes!(
                    "pixel",
                    set.then_some("on"),
                    ball.then_some("ball"),
                    (mem_addr < program_len).then_some("code")
                );
                html! { <span class={class}></span> }
            })
        });
        html! {
            <div class="monitor-bezel">
                <div class="screen-grid" style={format!("grid-template-columns: repeat({SCREEN_WIDTH}, {CELL_WIDTH}px); grid-auto-rows: {CELL_HEIGHT}px;")}>{ for pixels }</div>
            </div>
        }
    }

    fn view_listing(&self) -> Html {
        let current = self.machine.current_addr();
        let previous = self.machine.last_executed_addr;
        let rows = self.listing.lines().map(|line| {
            let addr = listing_addr(line).or_else(|| symbol_addr(line));
            html! {
                <span class={classes!(
                    (addr == current).then_some("current-line"),
                    (addr == previous && addr != current).then_some("previous-line"),
                )}>{ line }</span>
            }
        });
        html! { <pre class="listing">{ for rows }</pre> }
    }
}

fn listing_addr(line: &str) -> Option<u16> {
    let mut parts = line.split_whitespace();
    let _line_no = parts.next()?;
    let addr = parts.next()?;
    if addr.len() == 4 && addr.chars().all(|c| c.is_ascii_hexdigit()) {
        u16::from_str_radix(addr, 16).ok()
    } else {
        None
    }
}

fn axis_bucket(value: u8) -> u8 {
    ((value as u16 * 4) / 256) as u8
}

fn demo_value(kind: DemoKind) -> &'static str {
    match kind {
        DemoKind::Add => "add",
        DemoKind::Cassette => "cassette",
        DemoKind::Joystick => "joystick",
        DemoKind::Logo => "logo",
        DemoKind::Pattern => "pattern",
    }
}

fn demo_from_value(value: &str) -> Option<DemoKind> {
    match value {
        "add" => Some(DemoKind::Add),
        "cassette" => Some(DemoKind::Cassette),
        "joystick" => Some(DemoKind::Joystick),
        "logo" => Some(DemoKind::Logo),
        "pattern" => Some(DemoKind::Pattern),
        _ => None,
    }
}

fn initial_demo_from_query() -> Option<DemoKind> {
    let search = web_sys::window()?.location().search().ok()?;
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| (key == "demo").then(|| demo_from_value(value)).flatten())
}

fn scope_points(
    samples: &[crate::demo::ScopeSample],
    y_high: f64,
    y_low: f64,
    width: f64,
) -> String {
    if samples.is_empty() {
        return String::new();
    }

    let step = width / (samples.len().saturating_sub(1).max(1) as f64);
    let mut points = String::new();
    let mut previous_y = if samples[0].high { y_high } else { y_low };
    points.push_str(&format!("0.0,{previous_y:.1}"));

    for (idx, sample) in samples.iter().enumerate().skip(1) {
        let x = idx as f64 * step;
        let y = if sample.high { y_high } else { y_low };
        points.push_str(&format!(" {x:.1},{previous_y:.1}"));
        points.push_str(&format!(" {x:.1},{y:.1}"));
        previous_y = y;
    }
    points
}

fn cassette_scope_points(
    samples: &[crate::demo::CassetteScopeSample],
    y_high: f64,
    y_low: f64,
    width: f64,
) -> String {
    if samples.is_empty() {
        return String::new();
    }

    let step = width / (samples.len().saturating_sub(1).max(1) as f64);
    let mut points = String::new();
    let mut previous_y = if samples[0].high { y_high } else { y_low };
    points.push_str(&format!("0.0,{previous_y:.1}"));

    for (idx, sample) in samples.iter().enumerate().skip(1) {
        let x = idx as f64 * step;
        let y = if sample.high { y_high } else { y_low };
        points.push_str(&format!(" {x:.1},{previous_y:.1}"));
        points.push_str(&format!(" {x:.1},{y:.1}"));
        previous_y = y;
    }
    points
}

fn symbol_addr(line: &str) -> Option<u16> {
    let mut parts = line.split_whitespace();
    let _name = parts.next()?;
    let value = parts.next()?.strip_prefix("0x")?;
    u16::from_str_radix(value, 16).ok()
}

fn change_class(age: ChangeAge) -> &'static str {
    match age {
        ChangeAge::Recent => "changed-recent",
        ChangeAge::Older => "changed-older",
        ChangeAge::Stable => "changed-stable",
    }
}
