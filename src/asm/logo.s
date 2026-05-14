        ; One-shot CDP1802 logo drawing demo.
        ;
        ; Video scans the same 0x0000..0x00ff memory page that contains
        ; this program. The program intentionally writes pixels into
        ; already-executed bytes, then halts after the logo is visible.
        ORG 0x0000
START:  LDI 0x00
        PHI R1

        ; Top mark, inspired by the COSMAC ELF boot screen.
        LDI 0x00
        PLO R1
        LDI 0x7c
        STR R1
        LDI 0x01
        PLO R1
        LDI 0x82
        STR R1
        LDI 0x02
        PLO R1
        LDI 0xba
        STR R1
        LDI 0x03
        PLO R1
        LDI 0xaa
        STR R1
        LDI 0x04
        PLO R1
        LDI 0xba
        STR R1
        LDI 0x05
        PLO R1
        LDI 0x82
        STR R1
        LDI 0x06
        PLO R1
        LDI 0x7c
        STR R1

        ; Advance past the upper logo bytes before writing lower rows.
        ; These harmless one-byte instructions keep every draw target
        ; behind the current PC, so self-modifying video writes do not
        ; corrupt instructions that have not run yet.
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ
        REQ

        ; Block "ELF" in the lower half of the 256-byte video page.
        LDI 0x4a
        PLO R1
        LDI 0xf8
        STR R1
        LDI 0x4b
        PLO R1
        LDI 0x80
        STR R1
        LDI 0x4c
        PLO R1
        LDI 0xf8
        STR R1

        LDI 0x52
        PLO R1
        LDI 0x80
        STR R1
        LDI 0x53
        PLO R1
        LDI 0x80
        STR R1
        LDI 0x54
        PLO R1
        LDI 0x80
        STR R1

        LDI 0x5a
        PLO R1
        LDI 0xf0
        STR R1
        LDI 0x5b
        PLO R1
        LDI 0x80
        STR R1
        LDI 0x5c
        PLO R1
        LDI 0xf0
        STR R1

        LDI 0x62
        PLO R1
        LDI 0x80
        STR R1
        LDI 0x63
        PLO R1
        LDI 0x80
        STR R1
        LDI 0x64
        PLO R1
        LDI 0x80
        STR R1

        LDI 0x6a
        PLO R1
        LDI 0xf8
        STR R1
        LDI 0x6b
        PLO R1
        LDI 0xf8
        STR R1
        LDI 0x6c
        PLO R1
        LDI 0x80
        STR R1

DONE:   IDL
