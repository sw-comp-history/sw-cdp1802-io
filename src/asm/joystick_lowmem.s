        ; Compact 256-byte COSMAC ELF-II style joystick demo.
        ;
        ; The whole visible memory page is 256 bytes. This program is kept
        ; below one quarter of that page, so its bytes show up as noise in
        ; the top-left part of the display while most of the screen remains
        ; available for the ball.
        ;
        ; OUT 1 asks the web I/O board to clear display memory above the
        ; loaded program image. That preserves code bytes, including any
        ; accidental self-modification from moving the ball into code.
        ORG 0x0000
        OUT 1

        ; Pulse and sample Y first. The RC board exposes readiness through
        ; EF4; each B4 is one timing sample.
        OUT 3
        B4 Y0
        B4 Y1
        B4 Y2
        BR Y3

        ; Pulse and sample X from the selected Y bucket. The web I/O board
        ; records the measured bucket while the program performs the same
        ; polling sequence that a small hand-entered monitor program would.
Y0:     OUT 2
        B4 DRAW
        B4 DRAW
        B4 DRAW
        BR DRAW
Y1:     OUT 2
        B4 DRAW
        B4 DRAW
        B4 DRAW
        BR DRAW
Y2:     OUT 2
        B4 DRAW
        B4 DRAW
        B4 DRAW
        BR DRAW
Y3:     OUT 2
        B4 DRAW
        B4 DRAW
        B4 DRAW
        BR DRAW

        ; OUT 4 draws the ball from the measured X/Y buckets, then IDL
        ; returns control to the browser for the next joystick event.
DRAW:   OUT 4
        IDL
