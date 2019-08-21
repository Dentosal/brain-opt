# BrainOpt - Optimizing BrainFuck compiler

Compiles BrainFuck code to x86-64 assembly.
Quick compilation, and quick exection.
As if you would need those features with BrainFuck.

## Example: Hello World

Consider this rather beautiful `Hello World!` program:

```brainfuck
++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>
---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.
```

That gets optimized to: (manually commented)

```assembly
extern read
extern write
extern exit
global main

section .text
main:                           ; entry point
    sub rsp, 30000              ; allocate stack space
    mov rcx, 30000              ; set argument: count
    mov rdi, rsp                ; set argument: start
    xor al, al                  ; set argument: byte to clear with
    rep stosb                   ; zero allocated stack space
    mov rbx, rsp                ; set cell pointer
    sub rsp, 8                  ; align stack for extern calls
    mov rdi, 1                  ; set argument: fd = stdout
    mov rsi, constant_output0   ; set argument: buf = constant_output0
    mov rdx, 13                 ; set argument: count = 13
    call _write                 ; actually write to stdout
    xor rdi, rdi                ; set status code = 0
    call _exit                  ; exit

section .data
    constant_output0: db "Hello World!",0xa
```

It's rather well optimized fast, although cleaning 30000 bytes of stack is still not optimized away.


## Features

- [x] Deterministic builds
- [ ] CI tests for Linux (using Vagrant locally)

## Operating system support

- [x] Linux
- [x] MacOS
- [ ] Windows
