# sw-cdp1802-io

Live browser I/O demos for the RCA CDP1802 / COSMAC 1802 emulator.

Live demo: https://sw-comp-history.github.io/sw-cdp1802-io/

This repository is the web-facing companion to the CDP1802 emulator and assembler work in `sw-comp-history`. It provides a Yew/WASM application that visualizes period-style I/O devices while still running real CDP1802 code through the emulator.

## MVP: COSMAC ELF-II joystick and TV memory display

The first demo recreates a 1970s COSMAC ELF-II style experiment:

- A graphical joystick can be moved across an X/Y grid in the browser.
- Rust emulates the joystick potentiometers and resistor-capacitor timing circuit.
- A compact CDP1802 assembly program, under 64 bytes, clears non-code display memory, pulses output ports for the X and Y axes, polls `EF4`, and issues the draw command for the measured ball position.
- A simulated TV monitor renders memory as a 64 x 32 monochrome pixel grid.

The video behavior is intentionally historically rough. On the COSMAC ELF-II style setup, the video buffer could include ordinary memory, so the program bytes themselves appear as noise pixels. The joystick widget starts centered, and the initial ball lands near the center of the 256-byte memory display. If the ball moves into the part of the display backed by the running program, the program self-modifies and can crash on a later frame. That failure mode is part of the demo because it mirrors the behavior of the original hardware experiment.

## Repository layout

- `src/app.rs`: Yew UI with joystick, monitor, and CPU telemetry.
- `src/demo.rs`: CDP1802 machine wrapper that assembles the included source, runs the compact I/O protocol, tracks CPU status, and updates the 256-byte video page.
- `src/asm/joystick_lowmem.s`: the actual CDP1802 assembly program included with Rust `include_str!` and assembled at runtime.
- `styles/app.css`: application styling.
- `pages/`: tracked GitHub Pages output.
- `scripts/build-pages.sh`: Trunk build and `pages/` refresh script.
- `build-page.sh`: root wrapper for the Pages build.
- `.github/workflows/pages.yml`: Pages deploy workflow that publishes `./pages` after GitHub Actions is enabled.

## Run locally

Install the wasm target and Trunk if needed:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

Run the live development server:

```bash
trunk serve --open
```

## Build GitHub Pages output

```bash
./build-page.sh
```

The script builds the Yew/WASM application with Trunk and refreshes `pages/`, preserving `pages/.nojekyll` for GitHub Pages.

## Related repositories

- [`sw-cdp1802-emulator`](https://github.com/sw-comp-history/sw-cdp1802-emulator): CPU state, memory, execution, joystick RC board, and video helpers.
- [`sw-cdp1802-asm`](https://github.com/sw-comp-history/sw-cdp1802-asm): assembler used to assemble the included `.s` demo program.
- [`sw-cdp1802-isa`](https://github.com/sw-comp-history/sw-cdp1802-isa): CDP1802 instruction definitions and decode support.
- [`gen-isa`](https://github.com/sw-vibe-coding/gen-isa): scaffolder and multi-repo project layout documentation.

## License

MIT. See [`LICENSE`](LICENSE).
