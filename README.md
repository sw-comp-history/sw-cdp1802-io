# sw-cdp1802-io

Live browser I/O demos for the RCA CDP1802 / COSMAC 1802 emulator.

Live demo: https://sw-comp-history.github.io/sw-cdp1802-io/

![Logo demo screenshot](images/logo-screenshot.png?ts=1778866917642)

This repository is the web-facing companion to the CDP1802 emulator and assembler work in `sw-comp-history`. It provides a Yew/WASM application that visualizes period-style I/O devices while still running real CDP1802 code through the emulator.

## Demos: COSMAC ELF-II joystick and TV memory display

The live page opens on the **Logo** demo by default. Use the demo selector to switch to **Joystick** when you want to run the emulated RC timing experiment, or **Pattern** when you want to edit a small assembly program and run it in the browser.

The **Joystick** demo recreates a 1970s COSMAC ELF-II style experiment:

- A graphical joystick can be moved across an X/Y grid in the browser.
- Rust emulates the joystick potentiometers and resistor-capacitor timing circuit.
- A compact CDP1802 assembly program clears non-code display memory, pulses output ports for the X and Y axes, polls `EF4`, computes the video address, and writes the measured ball position itself.
- A simulated black-and-white TV monitor renders memory as a 64 x 32 bit grid, with each bit drawn as a taller light-gray block on black to approximate the stacked ELF-II video pixels.

The black-and-white video behavior is intentionally historically rough. On the COSMAC ELF-II style setup, the video buffer could include ordinary memory, so the program bytes themselves appear as noise pixels. The joystick widget starts centered, and the initial ball lands near the center of the 256-byte memory display. If the ball moves into the part of the display backed by the running program, the program self-modifies and can crash on a later frame. That failure mode is part of the demo because it mirrors the behavior of the original hardware experiment.

The **Logo** demo is manually entered static video data in the same 256-byte video page. There is no drawing code; reset executes a single `IDL` instruction while the monitor scans the initialized bytes as pixels. The demo selector switches the assembled source, listing, monitor, and CPU state view.

The **Pattern** demo is an editable CDP1802 assembly source panel. **Assemble** runs the Rust assembler on the current text and refreshes the listing and loaded memory image. **Run** assembles the current text, starts the emulator, and steps one 1802 instruction per browser timer callback so the page yields back to the browser thread between instructions. The default source writes a simple pattern into addresses `0x0080..0x0097`, which are the lower half of the 256-byte memory/video page.

## Repository layout

- `src/app.rs`: Yew UI with demo selector, conditional joystick controls, monitor, listing, and CPU telemetry.
- `src/demo.rs`: CDP1802 machine wrapper that assembles the selected source, runs the I/O protocol, tracks CPU status, and updates the 256-byte video page.
- `src/asm/joystick_lowmem.s`: joystick CDP1802 assembly source included with Rust `include_str!` and assembled at runtime.
- `src/asm/logo.s`: manually entered static logo video data included with Rust `include_str!` and assembled at runtime.
- `src/asm/pattern.s`: editable starter CDP1802 assembly source included with Rust `include_str!` and loaded into the Pattern demo.
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
