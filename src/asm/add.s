        ; Tiny step-through addition demo.
        ;
        ; Step 1 loads the first number into D. Step 2 adds the second
        ; number, leaving the sum in D. Step 3 halts.
        ORG 0x0000
START:  LDI 0x07
        ADI 0x05
        IDL
