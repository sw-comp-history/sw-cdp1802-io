use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;
use web_sys::MouseEvent;
use yew::prelude::*;

use crate::demo::{DemoMachine, SCREEN_HEIGHT, SCREEN_WIDTH, assembly_listing};

const PAD: f64 = 16.0;
const JOYSTICK_SIZE: f64 = 170.0;
const HANDLE_RADIUS: f64 = 9.0;
const CELL_WIDTH: usize = 4;
const CELL_HEIGHT: usize = 8;
const STEP_DELAY_MS: i32 = 120;

pub struct App {
    machine: DemoMachine,
    dragging: bool,
    listing: String,
    tick_pending: bool,
}

pub enum Msg {
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
        machine.run_frame(128, 128);
        Self {
            machine,
            dragging: false,
            listing: assembly_listing(),
            tick_pending: false,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::StartDrag(event) => {
                self.dragging = true;
                self.update_from_event(&event);
                self.schedule_tick(ctx);
                true
            }
            Msg::Drag(event) => {
                if self.dragging {
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
                        <h1>{"Joystick RC timing live demo"}</h1>
                    </div>
                    <div class="status-strip">
                        <span class={classes!("status-dot", self.machine.crashed.then_some("bad"))}></span>
                        <span>{ if self.machine.crashed { "self-modified code fault" } else { "running" } }</span>
                    </div>
                </header>

                <section class="demo-grid">
                    <div class="panel controls-panel">
                        <div class="panel-head">
                            <h2>{"Joystick"}</h2>
                            <button type="button" onclick={link.callback(|_| Msg::Reset)}>{"Reset"}</button>
                        </div>
                        { self.view_joystick(link) }
                        <div class="readouts">
                            <span>{ format!("X {:03}", self.machine.x) }</span>
                            <span>{ format!("Y {:03}", self.machine.y) }</span>
                            <span>{ format!("bucket {},{}", self.machine.x_bucket(), self.machine.y_bucket()) }</span>
                        </div>
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
                            <span>{ format!("{} bytes", self.machine.program_len) }</span>
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
        let x = ((event.offset_x() as f64 - PAD).clamp(0.0, span) / span * 255.0).round() as u8;
        let y = ((event.offset_y() as f64 - PAD).clamp(0.0, span) / span * 255.0).round() as u8;
        self.machine.start_frame(x, y);
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

    fn view_joystick(&self, link: &html::Scope<Self>) -> Html {
        let span = JOYSTICK_SIZE - PAD * 2.0;
        let cx = PAD + (self.machine.x as f64 / 255.0) * span;
        let cy = PAD + (self.machine.y as f64 / 255.0) * span;
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
                    <dd>{ format!("0x{:04x}", self.machine.last_state.read_reg(idx)) }</dd>
                </>
            }
        });
        html! {
            <section class="register-panel" aria-label="CPU registers">
                <div class="register-summary">
                    <span>{ format!("step {}", self.machine.last_steps) }</span>
                    <span>{ if self.machine.running() { "stepping" } else if self.machine.last_state.halted { "halted" } else { "ready" } }</span>
                    <span>{ self.machine.last_error.as_deref().unwrap_or("ok") }</span>
                </div>
                <dl class="control-registers">
                    <dt>{"PC"}</dt><dd>{ format!("0x{:04x}", self.machine.last_state.pc()) }</dd>
                    <dt>{"active"}</dt><dd>{ format!("0x{:04x}", self.machine.active_addr()) }</dd>
                    <dt>{"D"}</dt><dd>{ format!("0x{:02x}", self.machine.last_state.d) }</dd>
                    <dt>{"DF"}</dt><dd>{ self.machine.last_state.df.to_string() }</dd>
                    <dt>{"P"}</dt><dd>{ format!("R{:X}", self.machine.last_state.p) }</dd>
                    <dt>{"X"}</dt><dd>{ format!("R{:X}", self.machine.last_state.x) }</dd>
                    <dt>{"EF4"}</dt><dd>{ self.machine.last_state.ef[3].to_string() }</dd>
                    <dt>{"Q"}</dt><dd>{ self.machine.last_state.q.to_string() }</dd>
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
        let pixels = (0..SCREEN_HEIGHT).flat_map(|row| {
            let bytes = &bytes;
            (0..SCREEN_WIDTH).map(move |col| {
                let byte_index = row * (SCREEN_WIDTH / 8) + col / 8;
                let byte = bytes[byte_index];
                let set = (byte & (0x80 >> (col % 8))) != 0;
                let class = classes!(
                    "pixel",
                    set.then_some("on"),
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
            let addr = listing_addr(line);
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
