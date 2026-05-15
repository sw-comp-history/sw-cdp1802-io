        ; 4K cassette loader demo.
        ;
        ; This is the toggled-in program at 0x0000. The simulated
        ; cassette device appears on INP 4. Each INP 4 fetches one byte
        ; from the cassette stream and, because X selects R1, stores it
        ; into memory at R1. R1 starts at the video page 0x0100 and the
        ; loop stops after R1 reaches 0x0200, so 256 bytes are loaded.
        ORG 0x0000
START:  LDI 0x01
        PHI R1
        LDI 0x00
        PLO R1
        SEX R1

LOAD:   INP 4
        INC R1
        GHI R1
        XRI 0x02
        BNZ LOAD
        IDL
