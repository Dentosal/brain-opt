mod linux;
mod macos;

use strum_macros::{EnumString, EnumVariantNames};

use crate::codegen::{Instruction, Register64};

/// Instructions for linking
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkerInfo {
    /// Entry point symbol name, e.g. `main`
    pub entrypoint: String,
    /// Libraries to link against, e.g. `libc`
    pub libraries: Vec<String>,
    /// External symbols, e.g. `write`
    pub externs: Vec<String>,
    /// Object file format e.g. `elf64`
    pub object_format: String,
    /// Linker command, e.g. `gcc`
    pub linker_cmd: String,
    /// Linker extra arguments, e.g. `vec!["-no-pie"]`
    pub linker_args: Vec<String>,
}
impl LinkerInfo {
    /// Creates required assembly header
    pub fn to_assembly(&self) -> String {
        let mut r: String = self.externs.iter().map(|e| format!("extern {}\n", e)).collect();
        r.push_str(&format!("global {}\n", self.entrypoint));
        r
    }
}

pub trait Operations {
    /// Linker info
    fn linker_info(&self) -> LinkerInfo;

    /// Program startup code
    fn startup(&mut self) -> Vec<Instruction> {
        Vec::new()
    }

    /// Stop program execution with successful exit code
    fn exit(&mut self) -> Vec<Instruction>;

    /// Reads a single byte from stdin
    fn read_byte(&mut self, pointer: Register64) -> Vec<Instruction>;

    /// Writes `count` bytes to stdout
    fn write_bytes(&mut self, pointer: Register64, count: u64) -> Vec<Instruction>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum ABI {
    Linux,
    MacOS,
}
impl ABI {
    pub fn pick_default() -> Option<Self> {
        if cfg!(target_os = "linux") {
            Some(Self::Linux)
        } else if cfg!(target_os = "macos") {
            Some(Self::MacOS)
        } else {
            log::error!("Current platform not detected");
            None
        }
    }

    pub fn operations(self) -> Box<dyn Operations> {
        match self {
            Self::Linux => Box::new(linux::Interface::new()),
            Self::MacOS => Box::new(macos::Interface::new()),
        }
    }
}
