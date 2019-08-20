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
        let result = format!(".interface_macos{}", self.next_label);
        self.next_label += 1;
        result
    }
}
impl Operations for Interface {
    fn linker_info(&self) -> LinkerInfo {
        LinkerInfo {
            entrypoint: "_main".to_owned(),
            libraries: vec!["libc".to_owned()],
            externs: vec!["_read".to_owned(), "_write".to_owned(), "_exit".to_owned()],
            object_format: "macho64".to_owned(),
            linker_cmd: "ld".to_owned(),
            linker_args: vec![
                "-lSystem".to_owned(),
                "-macosx_version_min".to_owned(),
                "10.10.0".to_owned(),
            ],
        }
    }

    fn exit(&mut self) -> Vec<Instruction> {
        use Instruction::*;
        vec![
            MovImm(Register64::rdi, 0),
            BlackBox("call _exit".to_owned(), Effects {
                flags: true,
                registers: true,
                control_flow: true,
                stack: true,
                io: true,
            }),
        ]
    }

    fn read_byte(&mut self, pointer: Register64) -> Vec<Instruction> {
        use Instruction::*;
        let label_end = self.get_label();
        vec![
            MovImm(Register64::rdi, 0),
            Instruction::Mov(Register64::rsi, pointer),
            MovImm(Register64::rdx, 1),
            NamedBlackBox("read".to_owned(), "call _read".to_owned(), Effects {
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

    fn write_bytes(&mut self, pointer: Register64, count: u64) -> Vec<Instruction> {
        use Instruction::*;
        vec![
            MovImm(Register64::rdi, 1),
            Mov(Register64::rsi, pointer),
            MovImm(Register64::rdx, count),
            NamedBlackBox("write".to_owned(), "call _write".to_owned(), Effects {
                flags: true,
                registers: true,
                control_flow: false,
                stack: false,
                io: true,
            }),
        ]
    }
}
