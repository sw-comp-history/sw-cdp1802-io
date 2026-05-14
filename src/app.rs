use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::{Event, HtmlSelectElement, MouseEvent};
use yew::prelude::*;

use crate::demo::{
    ChangeAge, ControlField, DemoKind, DemoMachine, SCREEN_HEIGHT, SCREEN_WIDTH, listing_for,
};

const PAD: f64 = 16.0;
const JOYSTICK_SIZE: f64 = 170.0;
const HANDLE_RADIUS: f64 = 9.0;
const CELL_WIDTH: usize = 4;
const CELL_HEIGHT: usize = 8;
const STEP_DELAY_MS: i32 = 16;

pub struct App {
    machine: DemoMachine,
    dragging: bool,
    listing: String,
    tick_pending: bool,
    target_x: u8,
    target_y: u8,
}

pub enum Msg {
    SelectDemo(Event),
    StartDrag(MouseEvent),
    Drag(MouseEvent),
    StopDrag,
    Reset,
    Tick,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let mut machine = DemoMachine::default();
        machine.start_frame(128, 128);
        let listing = listing_for(machine.kind);
        Self {
            machine,
            dragging: false,
            listing,
            tick_pending: false,
            target_x: 128,
            target_y: 128,
        }
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
                let kind = match select.value().as_str() {
                    "logo" => DemoKind::Logo,
                    _ => DemoKind::Joystick,
                };
                self.target_x = 128;
                self.target_y = 128;
                self.machine.switch_demo(kind);
                self.listing = listing_for(kind);
                self.schedule_tick(ctx);
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
                self.machine.reset();
                self.machine.start_frame(128, 128);
                self.schedule_tick(ctx);
                true
            }
            Msg::Tick => {
                self.tick_pending = false;
                self.machine.step_frame();
                self.schedule_tick(ctx);
                true
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
                        <h1>{ format!("{} live demo", self.machine.kind.label()) }</h1>
                    </div>
                    <div class="status-strip">
                        <span class={classes!("status-dot", self.machine.crashed.then_some("bad"))}></span>
                        <span>{ if self.machine.crashed { "self-modified code fault" } else { "running" } }</span>
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
                        <p class="note">{ format!("Program image: 0x0000..0x{:04x}; video scans 0x0000..0x00ff.", self.machine.program_len.saturating_sub(1)) }</p>
                    </div>

                    <div class="panel listing-panel">
                        <div class="panel-head">
                            <h2>{"Assembler listing"}</h2>
                            <span>{ format!("{}: {} bytes", self.machine.kind.label(), self.machine.program_len) }</span>
                        </div>
                        { self.view_listing() }
                    </div>
                </section>
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
        html! {
            <select class="demo-select" onchange={link.callback(Msg::SelectDemo)} value={match self.machine.kind {
                DemoKind::Joystick => "joystick",
                DemoKind::Logo => "logo",
            }}>
                <option value="joystick">{"Joystick"}</option>
                <option value="logo">{"Logo"}</option>
            </select>
        }
    }

    fn view_demo_controls(&self, link: &html::Scope<Self>) -> Html {
        match self.machine.kind {
            DemoKind::Joystick => html! {
                <>
                    { self.view_joystick(link) }
                    <div class="readouts">
                        <span>{ format!("X {:03}", self.target_x) }</span>
                        <span>{ format!("Y {:03}", self.target_y) }</span>
                        <span>{ format!("bucket {},{}", axis_bucket(self.target_x), axis_bucket(self.target_y)) }</span>
                    </div>
                </>
            },
            DemoKind::Logo => html! {
                <div class="logo-demo-note">
                    <span>{"one-shot 1802 draw"}</span>
                    <span>{"video page 0x0000..0x00ff"}</span>
                </div>
            },
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
        let pixels = (0..SCREEN_HEIGHT).flat_map(|row| {
            let bytes = &bytes;
            (0..SCREEN_WIDTH).map(move |col| {
                let byte_index = row * (SCREEN_WIDTH / 8) + col / 8;
                let byte = bytes[byte_index];
                let set = (byte & (0x80 >> (col % 8))) != 0;
                let ball = set && ball_addr == Some(byte_index);
                let class = classes!(
                    "pixel",
                    set.then_some("on"),
                    ball.then_some("ball"),
                    (byte_index < program_len).then_some("code")
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
