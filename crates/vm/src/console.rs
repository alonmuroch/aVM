use crate::cpu::PrivilegeMode;
use crate::memory::{Memory, VirtualAddress};
use crate::metering::{MeterResult, Metering};
use core::fmt::Write;
use std::cell::RefCell;
use std::rc::Rc;
use std::string::String;
use std::vec::Vec;

pub const CONSOLE_WRITE_ID: u32 = 1000;

enum Arg {
    U32(u32),
    F32(f32),
    Char(char),
    Str(String),
    Bytes(Vec<u8>),
}

pub fn console_write(
    args: [u32; 6],
    caller_mode: PrivilegeMode,
    memory: Memory,
    metering: &mut dyn Metering,
    verbose_writer: &Option<Rc<RefCell<dyn Write>>>,
) -> u32 {
    let [fmt_ptr, fmt_len, arg_ptr, arg_len, ..] = args;
    let payload_len = fmt_len.saturating_add(arg_len) as usize;
    if matches!(
        metering.on_syscall_data(CONSOLE_WRITE_ID, payload_len),
        MeterResult::Halt
    ) {
        panic!("Metering halted console write");
    }
    let borrowed_memory = memory.as_ref();
    let (fmt_start, fmt_end) = va_range(fmt_ptr as usize, fmt_len as usize);
    let fmt_slice = match borrowed_memory.mem_slice(fmt_start, fmt_end) {
        Some(s) => s,
        None => {
            println!("invalid format string @ 0x{fmt_ptr:08x}");
            return 0;
        }
    };
    let fmt_bytes = fmt_slice.as_ref();
    let fmt = match core::str::from_utf8(fmt_bytes) {
        Ok(s) => s,
        Err(e) => {
            println!("invalid UTF-8 in format string");
            println!("bytes: {fmt_bytes:?}");
            println!("error: {e}");
            return 0;
        }
    };
    let (args_start, args_end) = va_range(arg_ptr as usize, arg_len as usize);
    let args_bytes_slice = borrowed_memory.mem_slice(args_start, args_end);
    let args_bytes_holder;
    let args_bytes: &[u8] = if let Some(slice) = args_bytes_slice {
        args_bytes_holder = slice;
        args_bytes_holder.as_ref()
    } else {
        b""
    };
    let raw_args: Vec<u32> = args_bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    let mut args: Vec<Arg> = Vec::new();
    let mut raw_iter = raw_args.into_iter();
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            continue;
        }
        let spec: char = chars.next().unwrap_or('%');
        let mut next = || raw_iter.next().unwrap_or(0);
        match spec {
            'd' | 'u' | 'x' => args.push(Arg::U32(next())),
            'f' => args.push(Arg::F32(f32::from_bits(next()))),
            'c' => args.push(Arg::Char(char::from_u32(next()).unwrap_or('?'))),
            's' => {
                let ptr = next() as usize;
                let len = next() as usize;
                let (start, end) = va_range(ptr, len);
                match borrowed_memory.mem_slice(start, end) {
                    Some(slice) => {
                        let s_ptr = core::str::from_utf8(slice.as_ref());
                        args.push(match s_ptr {
                            Ok(s) => Arg::Str(s.to_string()),
                            Err(_) => Arg::Str("<invalid>".to_string()),
                        });
                    }
                    None => {
                        args.push(Arg::Str("<invalid>".to_string()));
                    }
                }
            }
            'b' => {
                let ptr = next() as usize;
                let len = next() as usize;
                let (start, end) = va_range(ptr, len);
                match borrowed_memory.mem_slice(start, end) {
                    Some(slice) => {
                        args.push(Arg::Bytes(slice.to_vec()));
                    }
                    None => {
                        args.push(Arg::Str("<invalid>".to_string()));
                    }
                }
            }
            'a' => {
                let ptr = next() as usize;
                let len = next() as usize;
                let byte_len = len * 4;
                let (start, end) = va_range(ptr, byte_len);
                match borrowed_memory.mem_slice(start, end) {
                    Some(slice) => {
                        args.push(Arg::Bytes(slice.to_vec()));
                    }
                    None => {
                        args.push(Arg::Str("<invalid>".to_string()));
                    }
                }
            }
            'A' => {
                let ptr = next() as usize;
                let len = next() as usize;
                let (start, end) = va_range(ptr, len);
                match borrowed_memory.mem_slice(start, end) {
                    Some(slice) => {
                        args.push(Arg::Bytes(slice.to_vec()));
                    }
                    None => {
                        args.push(Arg::Str("<invalid>".to_string()));
                    }
                }
            }
            _ => args.push(Arg::Str("<bad-format>".to_string())),
        }
    }
    let mut output = String::new();
    let mut args_iter = args.iter();
    let mut fmt_chars = fmt.chars().peekable();
    while let Some(c) = fmt_chars.next() {
        if c == '%' {
            match fmt_chars.next() {
                Some('d') | Some('u') => match args_iter.next() {
                    Some(Arg::U32(v)) => output.push_str(&format!("{}", *v as i32)),
                    _ => output.push_str("<err>"),
                },
                Some('x') => match args_iter.next() {
                    Some(Arg::U32(v)) => output.push_str(&format!("{v:08x}")),
                    _ => output.push_str("<err>"),
                },
                Some('f') => match args_iter.next() {
                    Some(Arg::F32(f)) => output.push_str(&format!("{f}")),
                    _ => output.push_str("<err>"),
                },
                Some('c') => match args_iter.next() {
                    Some(Arg::Char(c)) => output.push(*c),
                    _ => output.push_str("<err>"),
                },
                Some('s') => match args_iter.next() {
                    Some(Arg::Str(s)) => output.push_str(s),
                    _ => output.push_str("<err>"),
                },
                Some('b') => match args_iter.next() {
                    Some(Arg::Bytes(b)) => {
                        output.push('[');
                        for (i, byte) in b.iter().enumerate() {
                            if i > 0 {
                                output.push_str(", ");
                            }
                            output.push_str(&format!("0x{byte:02x}"));
                        }
                        output.push(']');
                    }
                    _ => output.push_str("<err>"),
                },
                Some('a') => match args_iter.next() {
                    Some(Arg::Bytes(b)) => {
                        output.push('[');
                        for (i, chunk) in b.chunks_exact(4).enumerate() {
                            if i > 0 {
                                output.push_str(", ");
                            }
                            let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            output.push_str(&format!("{val}"));
                        }
                        output.push(']');
                    }
                    _ => output.push_str("<err>"),
                },
                Some('A') => match args_iter.next() {
                    Some(Arg::Bytes(b)) => {
                        output.push('[');
                        for (i, byte) in b.iter().enumerate() {
                            if i > 0 {
                                output.push_str(", ");
                            }
                            output.push_str(&format!("{byte}"));
                        }
                        output.push(']');
                    }
                    _ => output.push_str("<err>"),
                },
                Some('%') => output.push('%'),
                Some(_) | None => output.push_str("<%?>"),
            }
        } else {
            output.push(c);
        }
    }
    let _ = caller_mode;
    match verbose_writer {
        Some(writer) => {
            let _ = writeln!(writer.borrow_mut(), "{output}");
        }
        None => {
            println!("{output}");
        }
    }
    0
}

fn va_range(ptr: usize, len: usize) -> (VirtualAddress, VirtualAddress) {
    let start = VirtualAddress(ptr as u32);
    let end = start.wrapping_add(len as u32);
    (start, end)
}
