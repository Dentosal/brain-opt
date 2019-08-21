use crate::instruction::{Effects, Instruction, Register64};

use super::{LinkerInfo, Operations};

pub struct Interface {
    next_label: usize,
}
impl Interface {
    pub fn new() -> Self {
        Self { next_label: 0 }
    }

    fn get_label(&mut self) -> String {
        let result = format!(".interface_linux{}", self.next_label);
        self.next_label += 1;
        result
    }
}
impl Operations for Interface {
    fn linker_info(&self) -> LinkerInfo {
        LinkerInfo {
            entrypoint: "main".to_owned(),
            libraries: vec!["libc".to_owned()],
            externs: vec!["read".to_owned(), "write".to_owned(), "exit".to_owned()],
            object_format: "elf64".to_owned(),
            linker_cmd: "clang".to_owned(),
            linker_args: vec!["-no-pie".to_owned()],
        }
    }

    fn exit(&mut self) -> Vec<Instruction> {
        use Instruction::*;
        vec![
            BlackBox("add rsp, 30000".to_owned(), Effects {
                flags: true,
                registers: true,
                control_flow: true,
                stack: true,
                io: true,
            }),
            MovImm(Register64::rdi, 0),
            NamedBlackBox("exit".to_owned(), "call exit".to_owned(), Effects {
                flags: true,
                registers: true,
                control_flow: true,
                stack: true,
                io: true,
            }),
        ]
    }

    /// https://linux.die.net/man/2/read
    fn read_byte(&mut self, pointer: Register64) -> Vec<Instruction> {
        use Instruction::*;
        let label_end = self.get_label();
        vec![
            MovImm(Register64::rdi, 0),
            Instruction::Mov(Register64::rsi, pointer),
            MovImm(Register64::rdx, 1),
            BlackBox("call read".to_owned(), Effects {
                flags: true,
                registers: true,
                control_flow: false,
                stack: false,
                io: true,
            }),
            IsZero(Register64::rax),
            JumpNonZero(label_end.clone()),
            // End of file
            MovPtr8Imm(Register64::rsi, 0),
            Label(label_end),
        ]
    }

    /// https://linux.die.net/man/2/write
    fn write_bytes(&mut self, pointer: Register64, count: u64) -> Vec<Instruction> {
        use Instruction::*;
        vec![
            MovImm(Register64::rdi, 1),
            Mov(Register64::rsi, pointer),
            MovImm(Register64::rdx, count),
            NamedBlackBox("write".to_owned(), "call write".to_owned(), Effects {
                flags: true,
                registers: true,
                control_flow: false,
                stack: false,
                io: true,
            }),
        ]
    }
}
