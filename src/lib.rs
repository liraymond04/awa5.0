pub mod assembler;
pub mod compiler;
pub mod dynlib;
pub mod interpreter;
pub mod parser;

#[cfg(target_arch = "wasm32")]
mod awa5_raylib;

pub use assembler::*;
pub use compiler::*;
pub use dynlib::*;
pub use interpreter::*;
pub use parser::*;

use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum Awatism {
    Nop,
    Prn,
    Pr1,
    Red,
    R3d,
    Blo(u8), // s8
    Sbm(u8), // u5
    Pop,
    Dpl,
    Srn(u8),
    Mrg,
    Add,
    Sub,
    Mul,
    Div,
    Cnt,
    Lbl(u8),
    Jmp(u8),
    Eql,
    Lss,
    Gr8,
    Lib,
    Call(bool, String), // is_string, string
    Ret,
    Trm,
    // special awatism
    StrLbl(String),
    JmpRel,
    JmpRelStr(String),
}

impl Awatism {
    pub fn from_u8(value: u8, arg: u8) -> Option<Self> {
        match value {
            0x00 => Some(Awatism::Nop),
            0x01 => Some(Awatism::Prn),
            0x02 => Some(Awatism::Pr1),
            0x03 => Some(Awatism::Red),
            0x04 => Some(Awatism::R3d),
            0x05 => Some(Awatism::Blo(arg)),
            0x06 => Some(Awatism::Sbm(arg)),
            0x07 => Some(Awatism::Pop),
            0x08 => Some(Awatism::Dpl),
            0x09 => Some(Awatism::Srn(arg)),
            0x0a => Some(Awatism::Mrg),
            0x0b => Some(Awatism::Add),
            0x0c => Some(Awatism::Sub),
            0x0d => Some(Awatism::Mul),
            0x0e => Some(Awatism::Div),
            0x0f => Some(Awatism::Cnt),
            0x10 => Some(Awatism::Lbl(arg)),
            0x11 => Some(Awatism::Jmp(arg)),
            0x12 => Some(Awatism::Eql),
            0x13 => Some(Awatism::Lss),
            0x14 => Some(Awatism::Gr8),
            0x15 => None, // syscall
            0x16 => None, // double_pop
            0x17 => Some(Awatism::Lib),
            0x18 => Some(Awatism::JmpRel),
            0x19 => Some(Awatism::Call(false, "".to_string())),
            0x1A => Some(Awatism::Ret),
            0x1F => Some(Awatism::Trm),
            // special awatism
            _ => None,
        }
    }

    pub fn needs_args(value: u8) -> bool {
        match value {
            0x05 => true,
            0x06 => true,
            0x09 => true,
            0x10 => true,
            0x11 => true,
            _ => false,
        }
    }

    pub fn arg_bits(value: u8) -> usize {
        match value {
            0x05 => 8,
            0x06 => 5,
            0x09 => 5,
            0x10 => 5,
            0x11 => 5,
            _ => 0,
        }
    }
}

#[derive(Debug)]
pub struct Instruction {
    pub awatism: Awatism,
}

pub static AWA_SCII: &str = "AWawJELYHOSIUMjelyhosiumPCNTpcntBDFGRbdfgr0123456789 .,!'()~_/;\n";

pub fn detect_file_format(filename: &str) -> &str {
    let extension = Path::new(filename)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap();
    match extension {
        "awasm" => return "awasm",
        "awa" => return "awa",
        "awaml" => return "awaml",
        "o" => return "o",
        _ => {}
    }

    let content = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(_) => return "",
    };

    let normalized = content
        .chars()
        .filter(|&c| c == 'a' || c == 'w' || c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");

    if normalized.starts_with("awa ") {
        return "awa";
    }

    "awasm"
}

pub fn read_binary_file(filename: &str) -> Result<Vec<u8>, io::Error> {
    let file_content = fs::read(filename)?;
    Ok(file_content)
}

pub fn read_lines<P>(filename: P) -> io::Result<impl Iterator<Item = String>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines().filter_map(Result::ok))
}

pub fn write_object_file(filename: &str, vec: Vec<u8>) -> io::Result<()> {
    let mut file = File::create(filename).unwrap();

    file.write_all(&vec)
}

pub fn write_string_file(filename: &str, content: &str) -> io::Result<()> {
    let mut file = File::create(filename).unwrap();

    file.write_all(content.as_bytes())
}
