#include "p12f508.inc"

; CONFIG
; __config 0xFEA
 __CONFIG _OSC_IntRC & _WDT_OFF & _CP_OFF & _MCLRE_OFF


#define BUS_RX	 GP3
#define BUS_TX	 GP4

#define UART_RX  GP1
#define UART_TX  GP2

_A          equ 07h
_B          equ 08h
_C          equ 09h
_D          equ 0Ah

GPIO_BCKP   equ 0Bh
BIT_COUNT   equ 0Ch

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
; Message structure.
;
; 18                      12                               4               0
;  +-----------------------+-------------------------------+---------------+
;  |      message id       |            address            |   checksum    |
;  +-----------------------+-------------------------------+---------------+
;
; MESSAGE_0[0:4] -> checksum
; MESSAGE_0[4:8] -> address (low bits)
; MESSAGE_1[0:4] -> address (high bits)
; MESSAGE_1[4:8] -> message id (low bits)
; MESSAGE_2[0:2] -> message id (high bits)
;

MESSAGE     equ 10h
MESSAGE_0   equ 10h
MESSAGE_1   equ 11h
MESSAGE_2   equ 12h


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
;  Macro for waiting the specified numer of microseconds. The argument must be
;  765 at most.
;
;  Registers used: _C
;
wait_us MACRO us
    movlw   us/.3
    movwf   _C
    decfsz  _C, f
    goto    $-1
    ENDM

_sleep MACRO
    movfw    GPIO
    movwf    GPIO_BCKP
    sleep
    ENDM


RES_VECT  CODE   0x0000
    movwf   OSCCAL
    goto    START


MAIN_PROG CODE

START
    ; GP1 and GP3 are inputs, the rest are outputs.
    movlw   b'0001010'
    tris    GPIO

    ; Set options.
    ;
    ; bit 7    GPWU = 0     Enable wake-up on pin change bit (GP0, GP1, GP3).
    ; bit 6    GPPU = 0     Enable weak pull-ups bit (GP0, GP1, GP3).
    ; bit 5    T0CS = 0     Timer0 use internal instruction cycle clock (FOSC/4).
    ; bit 4    T0SE = 0
    ; bit 3    PSA  = 0     Pre-scaler assigned to Timer0.
    ; bit 2-0  PS   = 111   Pre-scaler rate 1 : 256.
    ;
    movlw   b'00000111'
    option

    bcf     GPIO, BUS_TX    ; Set BUT_TX low.

    btfsc   STATUS, GPWUF   ; GPWUF is set if the microcontroller has been reset
    goto    pin_changed     ; due to a pin change, if that's the case go ahead
                            ; and see which pin changed, if not go to sleep.
    _sleep

pin_changed:

    movfw   GPIO            ; After xoring GPIO with GPIO_BCKP the bits set to
    xorwf   GPIO_BCKP, f    ; 1 correspond to pins that changed.

    btfsc   GPIO_BCKP, BUS_RX  ; If change was in BUS_RX go to bus_rx.
    goto    bus_rx

    btfsc   GPIO_BCKP, UART_RX ; If change was in UART_RX go to uart_rx.
    goto    uart_rx

    _sleep

bus_rx:

    btfsc   GPIO, BUS_RX    ; Wait for BUS_RX to go low.
    goto    $-1

    clrf    TMR0            ; Now UART_RX is low, reset timer to 0.

    btfsc   GPIO, BUS_RX    ; Wait for BUS_RX to go high. When BUS_RX goes high
    goto    pulse_detected  ; the pulse has finished, go to pulse_detected.

    movlw   D'250'          ; If TMR0 > 250 then 64ms has elapsed without any
    subwf   TMR0, w         ; change in BUS_RX, in that case go to sleep. While
    btfss   STATUS, C       ; in sleep state a change in BUS_RX will wake-up
    goto    $-5             ; the MCU and execution will start at the entry
                            ; point.
    _sleep

pulse_detected:

    movf    TMR0, w         ; TMR0 now contains the amount of time that BUS_RX
    movwf   _A              ; remained in low state, store it in _A.

    movlw   D'8'            ; If it was less than ~2ms ignore the pulse by
    subwf   _A, w           ; going back to bus_rx. TMR0 is incremented
    btfss   STATUS, C       ; every 256 instruction cycles, each cycle is 1us,
    goto    bus_rx          ; 8 x 256us = 2048us -> ~2ms.

    movlw   D'16'           ; BUS_RX in low state during 3ms is a zero bit, we
    subwf   _A, w           ; leave room for inacuracies and consider a zero
    btfss   STATUS, C       ; every pulse between 2048us and 4096us (16 x 256).
    goto    bit_is_zero     ; Go and store the bit, at this point carry is 0.

    movlw   D'27'           ; 6ms means a one, every pulse between 4096us and
    subwf   _A, w           ; 6912us (27 x 256) is considered a one.
    btfss   STATUS, C
    goto    bit_is_one

    movlw   D'61'           ; A 17ms pulse indicates the start of a 18-bit
    subwf   _A, w           ; message, lets check if the pulse's length is
    btfss   STATUS, C       ; between 15616us (61 x 256) ....
    goto    bus_rx

    movlw   D'71'           ; ... and 18176us (71 x 256). If the pulse's length
    subwf   _A, w           ; is not in that range ignore the pulse and go wait
    btfsc   STATUS, C       ; for the next one.
    goto    bus_rx

    clrf    MESSAGE_0       ; If this point is reached the pulse is the 17ms
    clrf    MESSAGE_1       ; preamble indicating the start of a message, lets
    clrf    MESSAGE_2       ; clear the variables used for storing the message.

    clrf    BIT_COUNT       ; The bit count is also set to 0.

    goto    bus_rx          ; Wait for next pulse.

bit_is_one:

    bsf     STATUS, C       ; Set carry indicating that the received bit is a
                            ; one and add the bit to MESSAGE. When the bit is
                            ; zero we jump directly to bit_is_zero with carry
                            ; cleared.

bit_is_zero:                ; At this point the carry holds the value of the

    rlf     MESSAGE_0, f    ; received bit, rotate the message one bit to the
    rlf     MESSAGE_1, f    ; left, with the carry being added as the least
    rlf     MESSAGE_2, f    ; significant bit of MESSAGE_0.

    incf    BIT_COUNT, f    ; Increment BIT_COUNT.

    movlw   D'18'           ; If BIT_COUNT == 18 the whole message has been
    subwf   BIT_COUNT, w    ; read and we can proceed to verify its checksum,
    btfss   STATUS, Z       ; if not, go and wait for next pulse.
    goto    bus_rx

    clrf    _C              ; Set _C = 0, _C will contain the actual checksum,
                            ; which will be compared later with the expected
                            ; checksum received in the message.

    movlw   0xF0            ; Ignore the leftmost 4 bits from the message,
    andwf   MESSAGE_0, w    ; which contain the expected checksum, those bit
    call    count_ones      ; are not included in the checksum.

    movf    MESSAGE_1, w    ; The checksum is the number of bits that are set
    call    count_ones      ; to 1.

    movlw   0x03            ; From MESSAGE_2 only the 2 most significant bits
    andwf   MESSAGE_2, w    ; are taken into account.
    call    count_ones

    movlw   0x0F            ; Compare the 4 least significant bits of MESSAGE_0
    andwf   MESSAGE_0, w    ; (expected checksum) with _C (actual checksum). The
    call    reverse_bits    ; checksum bits in MESSAGE_0 are reversed, so they
                            ; must be reversed before comparing them with _C.
    xorwf   _C, w           ;
    btfsc   STATUS, Z       ; If they are equal the message is valid and will
    goto    uart_tx         ; be re-transmitted over UART, else go to sleep.

    _sleep

uart_tx:                    ; Transmit the content of message over UART, the
                            ; bits are sent in reverse order as they were
                            ; received from the bus.
    call    wait_3ms

    movfw   MESSAGE_2
    call    uart_tx_byte
    movfw   MESSAGE_1
    call    uart_tx_byte
    movfw   MESSAGE_0
    call    uart_tx_byte

    _sleep

uart_rx:

    clrf    MESSAGE_0       ; Clear message.
    clrf    MESSAGE_1
    clrf    MESSAGE_2

    movlw   3               ; Read 3 bytes from UART, the message is only 18
    movwf   _A              ; bits, but we need 3 bytes.

uart_rx_next_byte:

    movlw   8               ; Each byte is composed of 8 bits.
    movwf   BIT_COUNT

    wait_us .207            ; Skip start bit (which is always 0 and doesn't
                            ; contain actual data) and half the first bit so
                            ; that we sample UART_RX at the middle of each
                            ; bit. Each bit takes 208us at 4800 bauds.
uart_rx_next_bit:

    bcf     STATUS, C       ; Clear carry....
    btfsc   GPIO, UART_RX   ; and set it only if UART_RX is high, which
    bsf     STATUS, C       ; indicates a 1 being transmitted.

    rlf     MESSAGE_0, f    ; The carry contains the currently transmitted bit,
    rlf     MESSAGE_1, f    ; rotate the message one bit to the left and add
    rlf     MESSAGE_2, f    ; this new bit as the least significant bit.

    wait_us .201            ; Wait for the next bit.

    decfsz  BIT_COUNT, f    ; When BIT_COUNT is 0 we have read 8 bits.
    goto    uart_rx_next_bit

    wait_us .204            ; Skip the stop bit.

    decfsz  _A, f
    goto    uart_rx_next_byte  ; If the 3 bytes has been read, re-transmit the
                               ; the message over the bus.
bus_tx:

    call    pulse_train     ; Transmit preamble. The preamble should be 17ms
    call    wait_3ms        ; and we are waiting 18ms, but this doesn't make
    call    wait_3ms        ; a big difference.
    call    wait_3ms
    call    wait_3ms
    call    wait_3ms
    call    wait_3ms
    call    pulse_train

    movlw   b'00000010'
    movwf   _C

    movfw   MESSAGE_2
    call    bus_tx_bits

    movlw   b'10000000'
    movwf   _C

    movfw   MESSAGE_1
    call    bus_tx_bits

    movlw   b'10000000'
    movwf   _C

    movfw   MESSAGE_0
    call    bus_tx_bits

    _sleep

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
; Subroutine for counting the number of ones in a byte.
;
; This is the Brian Kerninghan's bit counting algorithm:
; https://graphics.stanford.edu/~seander/bithacks.html#CountBitsSetKernighan
;
; The byte is received in the W register, _C is incremented in the number
; of ones found in W, the caller must initialize _C accordingly. When the
; function exits _A is zero.
;
; Registers used: _A, _C.
;
count_ones:
    movwf  _A               ; _A = W
    iorlw  0
    btfsc  STATUS, Z        ; If W is 0.....
    retlw  0                ; ...nothing more to do

next_bit:
    incf   _C, f            ; _C = _C + 1
    decf   _A, w            ; W = _A - 1
    andwf  _A, f            ; _A = _A & W
    btfss  STATUS, Z        ; If A is not zero ...
    goto   next_bit         ; ... repeat
    retlw  1


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
; Subroutine that outputs a burst of pulses at 25 kHz during 3ms.
;
; Registers used: _A, _B.
;
pulse_train:
    movlw   D'75'           ; 75 pulses of 40us each -> 25 kHZ during 3ms.
    movwf   _A

next_pulse:
    movlw   D'6'
    movwf   _B

    bsf     GPIO, BUS_TX    ; Set BUS_TX to high.

    decfsz  _B, f           ; Remain 20us in high state. 6 loop iterations
    goto    $-1             ; take 18 instruction cycles (goto takes 2 cycles).

    movlw   D'5'            ; Plus 2 more instruction cycles.
    movwf   _B

    bcf	    GPIO, BUS_TX    ; Set BUS_TX to low.

    decfsz  _B, f           ; Remain 20us in low state. 5 loop iterations
    goto    $-1             ; take 15 instruction cyles (goto takes 2 cycles).


    decfsz  _A, f           ; Plus 5 more instruction cyles, 3 here and 2
    goto    next_pulse      ; due to the mov instructions after jumping to
                            ; next_pulse.
    retlw   0


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
; Subroutine that waits 3ms.
;
; Registers used: _A, _B.
;
wait_3ms:
    movlw   D'5'
    movwf   _A

    movlw   D'195'
    movwf   _B

    decfsz  _B, f
    goto    $-1

    decfsz  _A, f
    goto    $-5

    retlw   0


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
; Subroutine that transmits the byte in W over the bus. Only the bits indicated
; by _C are transmitted. For example, for transmitting all the bits in W, _C
; must be initialized with b'10000000', for the 7 least significant bits, _C
; must be b'01000000', and so on. _C must have exactly one bit set, if it's the
; N-th bit, the N least significant bits in W are transmitted.
;
; Registers used: _A (indirectly), _B (indirectly), _C, _D.
;
bus_tx_bits:

    movwf   _D              ; _D = W
    movfw   _C

bus_tx_next_bit:

    andwf   _D, w
    btfss   STATUS, Z	    ; If bit is zero the first call to wait_3ms is
    call    wait_3ms        ; skipped, resulting in a 3ms low pulse. If bit is
    call    wait_3ms        ; one the low pulse is 6ms long.
    call    pulse_train

    bcf     STATUS, C
    rrf     _C, f
    movf    _C, w
    btfss   STATUS, Z
    goto    bus_tx_next_bit ; When _C is zero there's no more bits to send.

    retlw   0

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
; Subroutine that transmits the byte in W at 4800 baud, 8 bits, no parity, via
; the UART_TX pin.
;
; Registers used: _A, _B, _C.
;
uart_tx_byte:

    movwf   _A              ; _A = W
    movlw   D'8'
    movwf   _B              ; _B = 8, the number of data bits being transmitted.

    bcf     GPIO, UART_TX   ; Transmit start bit.

    wait_us .204            ; The duration of each bit at 4800 baud is 208us,
                            ; wait for 204us, the next few instructions before
                            ; take the remaining 4us.
uart_tx_next_bit:

    rlf     _A, f           ; Move the most significant bit of _A to carry

    btfss   STATUS, C       ; If carry is 1 the next instruction is skipped and
    goto    $+3             ; TX is set high, if carry is 0 TX is set low.
    bsf     GPIO, UART_TX
    goto    $+2
    bcf     GPIO, UART_TX

    wait_us .201            ; Wait for 201us.

    decfsz  _B, f           ; When _B is 0 all bits have been transmitted.
    goto    uart_tx_next_bit

    bsf     GPIO, UART_TX   ; Transmit stop bit.

    wait_us .208

    retlw   0

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;
; Subroutine that reverses a nibble stored in W. It accepts values from 0x00
; to 0x0F. If input is 0b0001, output will be 0b1000.
;
reverse_bits:

    addwf   PCL
    retlw   0x00   ; 00
    retlw   0x08   ; 01
    retlw   0x04   ; 02
    retlw   0x0C   ; 03
    retlw   0x02   ; 04
    retlw   0x0A   ; 05
    retlw   0x06   ; 06
    retlw   0x0E   ; 07
    retlw   0x01   ; 08
    retlw   0x09   ; 09
    retlw   0x05   ; 0A
    retlw   0x0D   ; 0B
    retlw   0x03   ; 0C
    retlw   0x0B   ; 0D
    retlw   0x07   ; 0E
    retlw   0x0F   ; 0F

    END
