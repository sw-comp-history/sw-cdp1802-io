# CDP1802 I/O Web Demo Design

The first demo is a browser version of the COSMAC ELF-II joystick experiment.

- The Yew UI owns the draggable joystick widget and the monitor visualization.
- Rust models the joystick potentiometer and resistor-capacitor timing through `JoystickRcBoard` from `sw-cdp1802-emulator`.
- A CDP1802 program runs one frame at a time. It pulses output ports for Y and X, polls `EF4`, then stores one ball pixel.
- The monitor intentionally scans low memory, so program bytes are visible as noise pixels.
- Ball writes target low memory too. If the selected ball cell overlaps code, the program is self-modified and a later frame can fault. The UI reports that as part of the historical behavior.

Future devices can live beside this app as additional panels or routes: keypad, hex displays, Q LED, cassette, and alternate video mappings.
