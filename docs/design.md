# CDP1802 I/O Web Demo Design

The first demo is a browser version of the COSMAC ELF-II joystick experiment.

- The Yew UI owns the draggable joystick widget and the monitor visualization.
- Rust models the joystick potentiometer and resistor-capacitor timing through `JoystickRcBoard` from `sw-cdp1802-emulator`.
- A compact CDP1802 program runs one frame at a time. It clears non-code display memory, pulses output ports for Y and X, polls `EF4`, then issues the draw command for the measured ball position.
- The monitor intentionally scans the 256-byte memory page, so the sub-64-byte program image is visible as noise pixels in the top-left portion of the screen.
- Ball writes target low memory too. If the selected ball cell overlaps code, the program is self-modified and a later frame can fault. The UI reports that as part of the historical behavior.

Future devices can live beside this app as additional panels or routes: keypad, hex displays, Q LED, cassette, and alternate video mappings.
