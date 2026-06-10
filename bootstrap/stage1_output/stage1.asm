// Ajeeb bootstrap
.global _start
.text
_start:
    mov x1, sp
    adrp x0, __ajeeb_argv
    add x0, x0, :lo12:__ajeeb_argv
    str x1, [x0]
    bl fn_main
    mov x8, #93
    svc #0
fn_rdB:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_wrB:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_rdPos:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_wrPos:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_rdLbl:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_rdFnC:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_wrFnC:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_rdSrc:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_wrSrc:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_isDigit:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_isAlpha:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_isAlphaNum:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_isSpace:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_chr:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_skipWS:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_matchKwd:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_readIdent:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_addFn:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_collectFns:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_emitStr:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_emitI:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_emitFnBody:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
fn_main:
    stp x29, x30, [sp, #-16]!
    stp x19, x20, [sp, #-16]!
    str x21, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    mov x0, #0
    mov sp, x29
    ldr x21, [sp], #16
    ldp x19, x20, [sp], #16
    ldp x29, x30, [sp], #16
    ret
.section .bss
.align 4
__ajeeb_argv: .space 8
__ajeeb_buf: .space 16384
__ajeeb_itoa_buf: .space 32
__ajeeb_outbuf: .space 65536
