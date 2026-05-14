        ; Editable pattern demo.
        ;
        ; This tiny 1802 program writes three rows of bytes into the
        ; lower half of the 256-byte memory/video page. The TV monitor
        ; scans the same memory the CPU writes, so changing these LDI
        ; values changes the pixels on the screen.
        ORG 0x0000
START:  LDI 0x00        ; R1 points at video byte 0x0080.
        PHI R1
        LDI 0x80
        PLO R1

        ; Row 16: alternating vertical bars.
        LDI 0xaa
        STR R1
        INC R1
        LDI 0x55
        STR R1
        INC R1
        LDI 0xaa
        STR R1
        INC R1
        LDI 0x55
        STR R1
        INC R1
        LDI 0xaa
        STR R1
        INC R1
        LDI 0x55
        STR R1
        INC R1
        LDI 0xaa
        STR R1
        INC R1
        LDI 0x55
        STR R1
        INC R1

        ; Row 17: chunky blocks.
        LDI 0xf0
        STR R1
        INC R1
        LDI 0x0f
        STR R1
        INC R1
        LDI 0xf0
        STR R1
        INC R1
        LDI 0x0f
        STR R1
        INC R1
        LDI 0xf0
        STR R1
        INC R1
        LDI 0x0f
        STR R1
        INC R1
        LDI 0xf0
        STR R1
        INC R1
        LDI 0x0f
        STR R1
        INC R1

        ; Row 18: solid line.
        LDI 0xff
        STR R1
        INC R1
        STR R1
        INC R1
        STR R1
        INC R1
        STR R1
        INC R1
        STR R1
        INC R1
        STR R1
        INC R1
        STR R1
        INC R1
        STR R1

        IDL             ; Halt after drawing.
