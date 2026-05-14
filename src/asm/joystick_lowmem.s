        ; Shared COSMAC ELF-II 256-byte joystick RC demo.
        ;
        ; The full 0x0000..0x00ff memory page is also the 64x32 video
        ; display. Program bytes appear as pixels. If the ball lands on
        ; code, STR R1 overwrites code and the next frame can misbehave.
        ;
        ; OUT 1 asks the host board to clear display bytes above the
        ; loaded program image. OUT 3 pulses Y, OUT 2 pulses X, and EF4
        ; becomes true after the emulated RC delay. The 1802 counts the
        ; polling delay and computes the video address itself.
        ORG 0x0000
        OUT 1
        LDI SCRATCH
        PLO R2

        ; Unrolled Y polling loop. Each B4 is one EF4 input-pin sample;
        ; the label reached determines the row offset.
        OUT 3
        B4 Y0
        B4 Y1
        B4 Y2
        BR Y3

Y0:     LDI 0x00
        BR Y_DONE
Y1:     LDI 0x40
        BR Y_DONE
Y2:     LDI 0x80
        BR Y_DONE
Y3:     LDI 0xc0
Y_DONE:
        STR R2

        ; Unrolled X polling loop. These constants are byte offsets
        ; within an 8-byte video row: 0, 2, 4, or 6.
        OUT 2
        B4 X0
        B4 X1
        B4 X2
        BR X3

X0:     LDI 0x00
        BR X_DONE
X1:     LDI 0x02
        BR X_DONE
X2:     LDI 0x04
        BR X_DONE
X3:     LDI 0x06

        ; Add X byte offset to the stored Y row offset and write the
        ; ball pixel at the computed video address.
X_DONE:
        SEX R2
        ADD
        PLO R1
        LDI 0x80
        STR R1
        IDL

SCRATCH:
