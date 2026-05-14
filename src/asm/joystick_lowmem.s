        ; One animation frame for the web live I/O demo.
        ;
        ; Rust owns the joystick widget and runs this program once per
        ; joystick move. The display scans low memory, so these program
        ; bytes are visible as noise pixels. The ball target also lives in
        ; low memory; moving it into code self-modifies the program, which
        ; can crash later frames until Reset memory is pressed.
        ;
        ; OUT 3 pulses the emulated Y-axis RC circuit. Each B4 samples EF4
        ; once, selecting one of four Y buckets from the measured delay.
        ORG 0x0000
        OUT 3
        B4 Y0
        B4 Y1
        B4 Y2
        BR Y3

        ; For the selected Y bucket, pulse X with OUT 2 and select one of
        ; four X buckets using the same unrolled polling ladder.
Y0:     OUT 2
        B4 Y0X0
        B4 Y0X1
        B4 Y0X2
        BR Y0X3
Y1:     OUT 2
        B4 Y1X0
        B4 Y1X1
        B4 Y1X2
        BR Y1X3
Y2:     OUT 2
        B4 Y2X0
        B4 Y2X1
        B4 Y2X2
        BR Y2X3
Y3:     OUT 2
        B4 Y3X0
        B4 Y3X1
        B4 Y3X2
        BR Y3X3

        ; Redraw phase. Video page is 0x00 on purpose for the web demo.
        ; Row offsets are 0x00, 0x40, 0x80, 0xc0; columns are 0, 16,
        ; 32, and 48, encoded as byte offsets 0, 2, 4, and 6.
Y0X0:   LDI 0x00
        PHI R1
        LDI 0x00
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y0X1:   LDI 0x00
        PHI R1
        LDI 0x02
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y0X2:   LDI 0x00
        PHI R1
        LDI 0x04
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y0X3:   LDI 0x00
        PHI R1
        LDI 0x06
        PLO R1
        LDI 0x80
        STR R1
        BR DONE

Y1X0:   LDI 0x00
        PHI R1
        LDI 0x40
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y1X1:   LDI 0x00
        PHI R1
        LDI 0x42
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y1X2:   LDI 0x00
        PHI R1
        LDI 0x44
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y1X3:   LDI 0x00
        PHI R1
        LDI 0x46
        PLO R1
        LDI 0x80
        STR R1
        BR DONE

Y2X0:   LDI 0x00
        PHI R1
        LDI 0x80
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y2X1:   LDI 0x00
        PHI R1
        LDI 0x82
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y2X2:   LDI 0x00
        PHI R1
        LDI 0x84
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y2X3:   LDI 0x00
        PHI R1
        LDI 0x86
        PLO R1
        LDI 0x80
        STR R1
        BR DONE

Y3X0:   LDI 0x00
        PHI R1
        LDI 0xc0
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y3X1:   LDI 0x00
        PHI R1
        LDI 0xc2
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y3X2:   LDI 0x00
        PHI R1
        LDI 0xc4
        PLO R1
        LDI 0x80
        STR R1
        BR DONE
Y3X3:   LDI 0x00
        PHI R1
        LDI 0xc6
        PLO R1
        LDI 0x80
        STR R1

DONE:   IDL
