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
extern _read
extern _write
extern _exit
global _main

section .text
_main:
sub rsp, 30000
mov rcx, 30000
mov rdi, rsp
xor al, al
rep stosb
mov rbx, rsp
sub rsp, 8
mov rdi, 1
mov rsi, constant_output0
mov rdx, 13
call _write
xor rdi, rdi
call _exit
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
