use web_sys::MouseEvent;
use yew::prelude::*;

use crate::demo::{DemoMachine, SCREEN_HEIGHT, SCREEN_WIDTH};

const PAD: f64 = 22.0;
const JOYSTICK_SIZE: f64 = 260.0;
const HANDLE_RADIUS: f64 = 12.0;
const CELL: usize = 8;

pub struct App {
    machine: DemoMachine,
    dragging: bool,
}

pub enum Msg {
    StartDrag(MouseEvent),
    Drag(MouseEvent),
    StopDrag,
    Reset,
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
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::StartDrag(event) => {
                self.dragging = true;
                self.update_from_event(&event);
                true
            }
            Msg::Drag(event) => {
                if self.dragging {
                    self.update_from_event(&event);
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
                self.machine.run_frame(self.machine.x, self.machine.y);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <main class="app-shell">
                <section class="intro-band">
                    <div class="intro-copy">
                        <p class="eyebrow">{"RCA CDP1802 / COSMAC ELF-II I/O"}</p>
                        <h1>{"Live joystick RC timing demo"}</h1>
                        <p>{"Move the joystick. Rust emulates the analog RC delay, the CDP1802 program polls EF4, and the monitor scans memory as pixels."}</p>
                    </div>
                    <div class="status-strip">
                        <span class={classes!("status-dot", self.machine.crashed.then_some("bad"))}></span>
                        <span>{ if self.machine.crashed { "self-modified program crashed" } else { "program running" } }</span>
                    </div>
                </section>

                <section class="workbench">
                    <div class="panel joystick-panel">
                        <div class="panel-head">
                            <h2>{"Joystick"}</h2>
                            <button type="button" onclick={link.callback(|_| Msg::Reset)}>{"Reset memory"}</button>
                        </div>
                        { self.view_joystick(link) }
                        <div class="readouts">
                            <span>{ format!("X {:03}", self.machine.x) }</span>
                            <span>{ format!("Y {:03}", self.machine.y) }</span>
                            <span>{ format!("bucket {},{}", self.machine.x_bucket(), self.machine.y_bucket()) }</span>
                        </div>
                    </div>

                    <div class="panel monitor-panel">
                        <div class="panel-head">
                            <h2>{"TV monitor"}</h2>
                            <span>{"64 x 32 memory scan"}</span>
                        </div>
                        { self.view_monitor() }
                    </div>
                </section>

                <section class="telemetry">
                    <div>
                        <h2>{"CPU after last frame"}</h2>
                        <dl>
                            <dt>{"instructions"}</dt><dd>{ self.machine.last_steps }</dd>
                            <dt>{"halted"}</dt><dd>{ self.machine.last_state.halted.to_string() }</dd>
                            <dt>{"D"}</dt><dd>{ format!("0x{:02x}", self.machine.last_state.d) }</dd>
                            <dt>{"EF4"}</dt><dd>{ self.machine.last_state.ef[3].to_string() }</dd>
                            <dt>{"R0"}</dt><dd>{ format!("0x{:04x}", self.machine.last_state.read_reg(0)) }</dd>
                            <dt>{"R1"}</dt><dd>{ format!("0x{:04x}", self.machine.last_state.read_reg(1)) }</dd>
                            <dt>{"RF"}</dt><dd>{ format!("0x{:04x}", self.machine.last_state.read_reg(15)) }</dd>
                        </dl>
                    </div>
                    <div>
                        <h2>{"Historical behavior"}</h2>
                        <p>{"The display is intentionally pointed at low memory, where the demo program also lives. Program bytes appear as video noise. Ball writes can overwrite instructions, so a later frame may fault until memory is reset."}</p>
                        <p class="error-line">{ self.machine.last_error.as_deref().unwrap_or("No CPU fault on the last frame.") }</p>
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
        self.machine.run_frame(x, y);
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

    fn view_monitor(&self) -> Html {
        let bytes = self.machine.screen_bytes();
        let pixels = (0..SCREEN_HEIGHT).flat_map(|row| {
            let bytes = &bytes;
            (0..SCREEN_WIDTH).map(move |col| {
                let byte = bytes[row * (SCREEN_WIDTH / 8) + col / 8];
                let set = (byte & (0x80 >> (col % 8))) != 0;
                let class = if set { "pixel on" } else { "pixel" };
                html! {
                    <span class={class}></span>
                }
            })
        });
        html! {
            <div class="monitor-bezel">
                <div class="screen-grid" style={format!("grid-template-columns: repeat({SCREEN_WIDTH}, {CELL}px);")}>{ for pixels }</div>
            </div>
        }
    }
}
