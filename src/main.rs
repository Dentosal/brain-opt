use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use env_logger::Env;
use log::*;
use structopt::StructOpt;

use tempfile::tempdir;

use brain_opt::error::{Error, Result};
use brain_opt::ABI;
use brain_opt::{compile_tokens, parse};

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct Args {
    #[structopt(parse(from_os_str))]
    source: PathBuf,

    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    /// Save assembly code, give `-` to print to stdout
    #[structopt(short, long, parse(from_os_str))]
    assembly: Option<PathBuf>,

    /// Specify target ABI to use. Defaults to current OS ABI.
    #[structopt(short, long, raw(possible_values = "&ABI::variants()"))]
    target: Option<ABI>,

    /// Verbose mode (-v, -vv, -vvv)
    #[structopt(short, long, group = "verbosity", parse(from_occurrences))]
    verbose: u8,

    /// Quiet mode, no warnings
    #[structopt(short, long, group = "verbosity")]
    quiet: bool,
}
impl Args {
    pub fn verbosity_name(&self) -> &'static str {
        match self.verbose {
            0 if self.quiet => "error",
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }
    }
}

fn main() -> Result<()> {
    let args = Args::from_args();
    env_logger::from_env(Env::default().default_filter_or(args.verbosity_name())).init();

    let target_abi = args
        .target
        .or_else(ABI::pick_default)
        .ok_or(Error::UnknownTarget)?;
    info!("Selected target ABI: {:?}", target_abi);

    let source = fs::read(args.source)?;
    let tokens = parse(&String::from_utf8_lossy(&source));
    let (asm, link) = compile_tokens(tokens, target_abi);

    if let Some(out_asm) = args.assembly {
        if out_asm == Path::new("-") {
            println!("{}", asm);
        } else {
            fs::write(out_asm, asm.as_bytes())?;
        }
    }

    let dir = tempdir()?;
    let file_asm = dir.path().join("input.asm");
    let file_obj = dir.path().join("output.obj");

    fs::write(file_asm.clone(), asm.as_bytes())?;

    let status = Command::new("nasm")
        .arg("-f")
        .arg(link.object_format)
        .arg("-o")
        .arg(file_obj.clone())
        .arg(file_asm)
        .status()
        .expect("failed to execute nasm");

    if !status.success() {
        return Err(Error::Nasm);
    }

    let output_path = args.output.unwrap_or_else(|| {
        warn!("No output file specified, discarding executable");
        dir.path().join("output")
    });

    let mut linker = Command::new(link.linker_cmd);
    for arg in link.linker_args {
        linker.arg(arg);
    }
    linker
        .arg("-o")
        .arg(output_path)
        .arg(file_obj)
        .status()
        .expect("failed to execute linker");

    if !status.success() {
        return Err(Error::Linker);
    }

    Ok(())
}
