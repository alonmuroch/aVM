use core::cell::{Cell, RefCell};
use core::fmt::Write;
use std::any::Any;
use std::rc::Rc;

use state::State;
use types::{ADDRESS_LEN, address::Address};
use vm::memory::{API, MMU, HEAP_PTR_OFFSET, Memory, Perms, VirtualAddress};
use vm::metering::{MemoryAccessKind, MeterResult, Metering, NoopMeter};
use vm::registers::Register;
use vm::sys_call::{
    SYSCALL_ALLOC, SYSCALL_BALANCE, SYSCALL_BRK, SYSCALL_CALL_PROGRAM, SYSCALL_DEALLOC,
    SYSCALL_FIRE_EVENT, SYSCALL_LOG, SYSCALL_PANIC, SYSCALL_STORAGE_GET, SYSCALL_STORAGE_SET,
    SYSCALL_TRANSFER, SyscallHandler,
};

/// Represents different types of arguments that can be passed to system calls.
///
/// EDUCATIONAL: This enum demonstrates how to handle different data types
/// in system calls. In real operating systems, system calls need to handle
/// various data types safely.
#[allow(dead_code)]
enum Arg {
    U32(u32),       // 32-bit unsigned integer
    F32(f32),       // 32-bit floating point
    Char(char),     // Single character
    Str(String),    // String (owned)
    Bytes(Vec<u8>), // Raw bytes
}

pub struct DefaultSyscallHandler {
    verbose_writer: Option<Rc<RefCell<dyn Write>>>,
    state: Rc<RefCell<State>>,
    heap_ptr: Rc<Cell<u32>>,
}

impl std::fmt::Debug for DefaultSyscallHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultSyscallHandler")
            .field(
                "verbose_writer",
                &self.verbose_writer.as_ref().map(|_| "<dyn Write>"),
            )
            .field("state", &"<State>")
            .finish()
    }
}

impl DefaultSyscallHandler {
    fn ensure_heap_ptr(&self) -> u32 {
        let current = self.heap_ptr.get();
        if current == 0 {
            self.heap_ptr.set(HEAP_PTR_OFFSET);
            HEAP_PTR_OFFSET
        } else {
            current
        }
    }

    fn set_heap_ptr(&self, next: u32) {
        self.heap_ptr.set(next);
    }

    fn write_bytes(&self, memory: &Memory, start: VirtualAddress, data: &[u8]) -> bool {
        let mut meter = NoopMeter::default();
        for (idx, byte) in data.iter().enumerate() {
            let addr = start.wrapping_add(idx as u32);
            if !memory.store_u8(addr, *byte, &mut meter, MemoryAccessKind::Store) {
                return false;
            }
        }
        true
    }

    fn alloc_on_heap(
        &self,
        memory: &Memory,
        data: &[u8],
        align: u32,
    ) -> Option<VirtualAddress> {
        let mut addr = self.ensure_heap_ptr();
        let mask = align.checked_sub(1)?;
        addr = addr.checked_add(mask)? & !mask;
        let end = addr.checked_add(data.len() as u32)?;
        let start = VirtualAddress(addr);
        memory.map_range(start, data.len(), Perms::rw_kernel());
        if !data.is_empty() && !self.write_bytes(memory, start, data) {
            return None;
        }
        self.set_heap_ptr(end);
        Some(start)
    }
    pub fn new(state: Rc<RefCell<State>>) -> Self {
        Self::with_writer_and_heap(state, None, Rc::new(Cell::new(0)))
    }

    pub fn with_heap(state: Rc<RefCell<State>>, heap_ptr: Rc<Cell<u32>>) -> Self {
        Self::with_writer_and_heap(state, None, heap_ptr)
    }

    pub fn with_writer(
        state: Rc<RefCell<State>>,
        writer: Option<Rc<RefCell<dyn Write>>>,
    ) -> Self {
        Self::with_writer_and_heap(state, writer, Rc::new(Cell::new(0)))
    }

    fn with_writer_and_heap(
        state: Rc<RefCell<State>>,
        writer: Option<Rc<RefCell<dyn Write>>>,
        heap_ptr: Rc<Cell<u32>>,
    ) -> Self {
        Self {
            verbose_writer: writer,
            state,
            heap_ptr,
        }
    }
}

impl SyscallHandler for DefaultSyscallHandler {
    fn handle_syscall(
        &mut self,
        call_id: u32,
        args: [u32; 6],
        caller_mode: vm::cpu::PrivilegeMode,
        memory: Memory,
        regs: &mut [u32; 32],
        metering: &mut dyn Metering,
    ) -> (u32, bool) {
        if matches!(metering.on_syscall(call_id, &args), MeterResult::Halt) {
            panic!("Metering halted syscall {}", call_id);
        }
        let result = match call_id {
            SYSCALL_STORAGE_GET => self.sys_storage_get(args, memory, metering),
            SYSCALL_STORAGE_SET => self.sys_storage_set(args, memory, metering),
            SYSCALL_PANIC => self.sys_panic_with_message(regs, memory),
            SYSCALL_LOG => self.sys_log(args, caller_mode, memory, metering),
            SYSCALL_CALL_PROGRAM => self.sys_call_program(args, memory, metering),
            SYSCALL_FIRE_EVENT => self.sys_fire_event(args, memory, metering),
            SYSCALL_ALLOC => self.sys_alloc(args, memory, metering),
            SYSCALL_DEALLOC => self.sys_dealloc(args, memory, metering),
            SYSCALL_TRANSFER => self.sys_transfer(args, memory, metering),
            SYSCALL_BALANCE => self.sys_balance(args, memory, metering),
            SYSCALL_BRK => self.sys_brk(args, memory, metering),
            _ => {
                panic!("Unknown syscall: {}", call_id);
            }
        };
        (result, true)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl DefaultSyscallHandler {
    pub fn sys_fire_event(
        &mut self,
        args: [u32; 6],
        memory: Memory,
        metering: &mut dyn Metering,
    ) -> u32 {
        // EDUCATIONAL: Extract key pointer and length from arguments
        let ptr = args[0] as usize;
        let len = args[1] as usize;

        if matches!(
            metering.on_syscall_data(SYSCALL_FIRE_EVENT, len),
            MeterResult::Halt
        ) {
            panic!("Metering halted SYSCALL_FIRE_EVENT");
        }

        let borrowed_memory = memory.as_ref();

        // EDUCATIONAL: Safely read the key from memory
        // EDUCATIONAL: Create a limited scope to avoid borrow checker issues
        let (start, end) = va_range(ptr, len);
        let _event_bytes = match borrowed_memory.mem_slice(start, end) {
            Some(r) => r,
            None => panic!("invalid memory access"), // Invalid memory access
        };

        0
    }

    fn sys_storage_get(
        &mut self,
        args: [u32; 6],
        memory: Memory,
        metering: &mut dyn Metering,
    ) -> u32 {
        let address_ptr = args[0] as usize;
        let domain_ptr = args[1] as usize;
        let key_ptr = args[2] as usize;
        let lens_packed = args[3] as usize;
        let domain_len = lens_packed & 0xffff;
        let key_len = lens_packed >> 16;

        let total_len = ADDRESS_LEN
            .saturating_add(domain_len)
            .saturating_add(key_len);
        if matches!(
            metering.on_syscall_data(SYSCALL_STORAGE_GET, total_len),
            MeterResult::Halt
        ) {
            panic!("Metering halted SYSCALL_STORAGE_GET");
        }

        let borrowed_memory = memory.as_ref();

        // Parse address
        let (address_start, address_end) = va_range(address_ptr, ADDRESS_LEN);
        let address_slice_ref = match borrowed_memory.mem_slice(address_start, address_end) {
            Some(r) => r,
            None => {
                println!(
                    "❌ Storage GET - Invalid address memory access: ptr={}, len={}",
                    address_ptr, ADDRESS_LEN
                );
                return 0;
            }
        };
        let address_bytes = address_slice_ref.as_ref();
        let address_hex = address_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("");
        let mut addr_arr = [0u8; ADDRESS_LEN];
        addr_arr.copy_from_slice(address_bytes);
        let address = Address(addr_arr);

        // Parse domain
        let domain_slice = {
            let (domain_start, domain_end) = va_range(domain_ptr, domain_len);
            let domain_slice_ref = match borrowed_memory.mem_slice(domain_start, domain_end) {
                Some(r) => r,
                None => {
                    println!(
                        "❌ Storage GET - Invalid domain memory access: ptr={}, len={}",
                        domain_ptr, domain_len
                    );
                    return 0;
                }
            };
            domain_slice_ref.as_ref().to_vec()
        };
        let domain = match core::str::from_utf8(&domain_slice) {
            Ok(s) => s,
            Err(_) => {
                println!(
                    "❌ Storage GET - Invalid UTF-8 in domain: {:?}",
                    domain_slice
                );
                return 0;
            }
        };

        // Parse key
        let key_slice = {
            let (key_start, key_end) = va_range(key_ptr, key_len);
            let key_slice_ref = match borrowed_memory.mem_slice(key_start, key_end) {
                Some(r) => r,
                None => {
                    println!(
                        "❌ Storage GET - Invalid key memory access: ptr={}, len={}",
                        key_ptr, key_len
                    );
                    return 0;
                }
            };
            key_slice_ref.as_ref().to_vec()
        };
        // Convert binary key to hex string for storage
        let key = key_slice
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("");

        // Format key for display based on domain
        let display_key = if domain == "P" {
            // For persistent domain, try to display key as ASCII
            match core::str::from_utf8(&key_slice) {
                Ok(s) => s.to_string(),
                Err(_) => key.clone(), // fallback to hex if not valid UTF-8
            }
        } else {
            // For other domains, show domain as ASCII and key as hex
            format!("{}:{}", domain, key)
        };

        let value = {
            let state_ref = self.state.borrow();
            state_ref
                .get_account(&address)
                .and_then(|acc| acc.storage.get(&format!("{}:{}", domain, key)).cloned())
        };

        if let Some(value) = value {
            let mut buf = (value.len() as u32).to_le_bytes().to_vec();
            buf.extend_from_slice(value.as_slice());
            if matches!(metering.on_alloc(buf.len()), MeterResult::Halt) {
                panic!("Metering halted alloc during storage_get");
            }
            let addr = match self.alloc_on_heap(&memory, &buf, 8) {
                Some(ptr) => ptr,
                None => return 0,
            };
            println!(
                "✅ Found value for address: '{}', domain: '{}', Key: '{}'",
                address_hex, domain, display_key
            );
            return addr.as_u32();
        } else {
            println!(
                "❌ No value found for address: '{}', domain: '{}', key: '{}'",
                address_hex, domain, display_key
            );
            0
        }
    }

    fn sys_storage_set(
        &mut self,
        args: [u32; 6],
        memory: Memory,
        metering: &mut dyn Metering,
    ) -> u32 {
        let address_ptr = args[0] as usize;
        let domain_ptr = args[1] as usize;
        let key_ptr = args[2] as usize;
        let lens_packed = args[3] as usize;
        let val_ptr = args[4] as usize;
        let val_len = args[5] as usize;

        let domain_len = lens_packed & 0xffff;
        let key_len = lens_packed >> 16;

        let total_len = ADDRESS_LEN
            .saturating_add(domain_len)
            .saturating_add(key_len)
            .saturating_add(val_len);
        if matches!(
            metering.on_syscall_data(SYSCALL_STORAGE_SET, total_len),
            MeterResult::Halt
        ) {
            panic!("Metering halted SYSCALL_STORAGE_SET");
        }

        let borrowed_memory = memory.as_ref();

        // Parse address
        let (address_start, address_end) = va_range(address_ptr, ADDRESS_LEN);
        let address_slice_ref = match borrowed_memory.mem_slice(address_start, address_end) {
            Some(r) => r,
            None => {
                println!(
                    "❌ Storage SET - Invalid address memory access: ptr={}, len={}",
                    address_ptr, ADDRESS_LEN
                );
                return 0;
            }
        };
        let address_bytes = address_slice_ref.as_ref();
        let address_hex = address_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("");
        let mut addr_arr = [0u8; ADDRESS_LEN];
        addr_arr.copy_from_slice(address_bytes);
        let address = Address(addr_arr);

        // Parse domain
        let (domain_start, domain_end) = va_range(domain_ptr, domain_len);
        let domain_slice_ref = match borrowed_memory.mem_slice(domain_start, domain_end) {
            Some(r) => r,
            None => {
                println!(
                    "❌ Storage SET - Invalid domain memory access: ptr={}, len={}",
                    domain_ptr, domain_len
                );
                return 0;
            }
        };
        let domain_slice = domain_slice_ref.as_ref();
        let domain = match core::str::from_utf8(domain_slice) {
            Ok(s) => s,
            Err(_) => {
                println!(
                    "❌ Storage SET - Invalid UTF-8 in domain: {:?}",
                    domain_slice
                );
                return 0;
            }
        };

        // Parse key
        let (key_start, key_end) = va_range(key_ptr, key_len);
        let key_slice_ref = match borrowed_memory.mem_slice(key_start, key_end) {
            Some(r) => r,
            None => {
                println!(
                    "❌ Storage SET - Invalid key memory access: ptr={}, len={}",
                    key_ptr, key_len
                );
                return 0;
            }
        };
        let key_slice = key_slice_ref.as_ref();
        // Convert binary key to hex string for storage
        let key = key_slice
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("");

        // Format key for display based on domain
        let display_key = if domain == "P" {
            // For persistent domain, try to display key as ASCII
            match core::str::from_utf8(key_slice) {
                Ok(s) => s.to_string(),
                Err(_) => key.clone(), // fallback to hex if not valid UTF-8
            }
        } else {
            // For other domains, show domain as ASCII and key as hex
            format!("{}:{}", domain, key)
        };

        // Parse value
        let (val_start, val_end) = va_range(val_ptr, val_len);
        let value_slice_ref = match borrowed_memory.mem_slice(val_start, val_end) {
            Some(r) => r,
            None => {
                println!(
                    "❌ Storage SET - Invalid value memory access: ptr={}, len={}",
                    val_ptr, val_len
                );
                return 0;
            }
        };
        let value_slice = value_slice_ref.as_ref();

        println!(
            "💾 Storage SET - Domain: '{}', Key: '{}', Value: {:?} ({} bytes)",
            domain,
            display_key,
            value_slice,
            value_slice.len()
        );

        let composite_key = format!("{}:{}", domain, key);
        self.state
            .borrow_mut()
            .get_account_mut(&address)
            .storage
            .insert(composite_key, value_slice.to_vec());
        0
    }

    fn sys_panic_with_message(&mut self, regs: &mut [u32; 32], memory: Memory) -> u32 {
        let msg_ptr = regs[Register::A0 as usize] as usize;
        let msg_len = regs[Register::A1 as usize] as usize;
        let (msg_start, msg_end) = va_range(msg_ptr, msg_len);
        let msg = memory
            .mem_slice(msg_start, msg_end)
            .map(|bytes| String::from_utf8_lossy(bytes.as_ref()).into_owned())
            .unwrap_or_else(|| "<invalid memory access>".to_string());
        panic!("🔥 Guest panic: {}", msg);
    }

    fn sys_log(
        &mut self,
        args: [u32; 6],
        caller_mode: vm::cpu::PrivilegeMode,
        memory: Memory,
        metering: &mut dyn Metering,
    ) -> u32 {
        let [fmt_ptr, fmt_len, arg_ptr, arg_len, ..] = args;
        let payload_len = fmt_len.saturating_add(arg_len) as usize;
        if matches!(
            metering.on_syscall_data(SYSCALL_LOG, payload_len),
            MeterResult::Halt
        ) {
            panic!("Metering halted SYSCALL_LOG");
        }
        let borrowed_memory = memory.as_ref();
        let (fmt_start, fmt_end) = va_range(fmt_ptr as usize, fmt_len as usize);
        let fmt_slice = match borrowed_memory.mem_slice(fmt_start, fmt_end) {
            Some(s) => s,
            None => {
                println!("⚠️ invalid format string @ 0x{:08x}", fmt_ptr);
                return 0;
            }
        };
        let fmt_bytes = fmt_slice.as_ref();
        let fmt = match core::str::from_utf8(fmt_bytes) {
            Ok(s) => s,
            Err(e) => {
                println!("⚠️ invalid UTF-8 in format string");
                println!("📦 bytes: {:?}", fmt_bytes);
                println!("❌ error: {}", e);
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
                    // Array of u32s
                    let ptr = next() as usize;
                    let len = next() as usize;
                    let byte_len = len * 4; // u32 is 4 bytes
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
                    // Array of u8s
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
                        Some(Arg::U32(v)) => output.push_str(&format!("{:08x}", v)),
                        _ => output.push_str("<err>"),
                    },
                    Some('f') => match args_iter.next() {
                        Some(Arg::F32(f)) => output.push_str(&format!("{}", f)),
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
                            // Format bytes array nicely
                            output.push('[');
                            for (i, byte) in b.iter().enumerate() {
                                if i > 0 {
                                    output.push_str(", ");
                                }
                                output.push_str(&format!("0x{:02x}", byte));
                            }
                            output.push(']');
                        }
                        _ => output.push_str("<err>"),
                    },
                    Some('a') => match args_iter.next() {
                        Some(Arg::Bytes(b)) => {
                            // Format u32 array (bytes interpreted as u32s)
                            output.push('[');
                            for (i, chunk) in b.chunks_exact(4).enumerate() {
                                if i > 0 {
                                    output.push_str(", ");
                                }
                                let val =
                                    u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                                output.push_str(&format!("{}", val));
                            }
                            output.push(']');
                        }
                        _ => output.push_str("<err>"),
                    },
                    Some('A') => match args_iter.next() {
                        Some(Arg::Bytes(b)) => {
                            // Format u8 array
                            output.push('[');
                            for (i, byte) in b.iter().enumerate() {
                                if i > 0 {
                                    output.push_str(", ");
                                }
                                output.push_str(&format!("{}", byte));
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
        let prefix = match caller_mode {
            vm::cpu::PrivilegeMode::Supervisor => "🛡️ Kernel",
            vm::cpu::PrivilegeMode::User => "📜 Guest",
        };
        match &self.verbose_writer {
            Some(writer) => {
                let _ = writeln!(writer.borrow_mut(), "{}: {}", prefix, output);
            }
            None => {
                println!("{}: {}", prefix, output);
            }
        }
        0
    }

    fn sys_call_program(
        &mut self,
        args: [u32; 6],
        memory: Memory,
        metering: &mut dyn Metering,
    ) -> u32 {
        let to_ptr = args[0] as usize;
        let from_ptr = args[1] as usize;
        let input_ptr = args[2] as usize;
        let input_len = args[3] as usize;
        if matches!(metering.on_call(input_len), MeterResult::Halt) {
            panic!("Metering halted SYSCALL_CALL_PROGRAM");
        }
        {
            let borrowed_memory = memory.as_ref();
            let (to_start, to_end) = va_range(to_ptr, 20);
            let _to_slice = match borrowed_memory.mem_slice(to_start, to_end) {
                Some(r) => r,
                None => return 0,
            };
            let (from_start, from_end) = va_range(from_ptr, 20);
            let _from_slice = match borrowed_memory.mem_slice(from_start, from_end) {
                Some(r) => r,
                None => return 0,
            };
            let (input_start, input_end) = va_range(input_ptr, input_len);
            let _input_slice = match borrowed_memory.mem_slice(input_start, input_end) {
                Some(r) => r,
                None => return 0,
            };
        }
        // Host integration removed; call_program is currently unsupported.
        0
    }

    fn sys_alloc(&mut self, args: [u32; 6], memory: Memory, metering: &mut dyn Metering) -> u32 {
        let size = args[0] as usize; // A0 register
        let align = args[1] as usize; // A1 register

        if matches!(metering.on_alloc(size), MeterResult::Halt) {
            panic!("Metering halted SYSCALL_ALLOC");
        }

        if size == 0 {
            println!("VM Alloc: Invalid size 0");
            return 0;
        }

        // Validate alignment (must be power of 2)
        if align == 0 || (align & (align - 1)) != 0 {
            println!("VM Alloc: Invalid alignment {}", align);
            return 0;
        }

        // Allocate aligned memory on heap
        let data = vec![0u8; size];
        let ptr = match self.alloc_on_heap(&memory, &data, align as u32) {
            Some(ptr) => ptr,
            None => {
                println!("VM Alloc: Out of memory, failed to allocate {} bytes", size);
                return 0;
            }
        };

        ptr.as_u32()
    }

    fn sys_dealloc(&mut self, args: [u32; 6], _memory: Memory, metering: &mut dyn Metering) -> u32 {
        let size = args[1] as usize;
        if matches!(metering.on_alloc(size), MeterResult::Halt) {
            panic!("Metering halted SYSCALL_DEALLOC");
        }
        // Note: This VM uses a simple bump allocator, so we can't actually free memory
        // In a real VM, you'd implement a proper allocator with free lists
        // For now, this is a no-op since the memory will be reclaimed when the VM exits
        0
    }

    fn sys_transfer(
        &mut self,
        args: [u32; 6],
        memory: Memory,
        metering: &mut dyn Metering,
    ) -> u32 {
        // args: a2=to ptr, a3=value_lo, a4=value_hi
        let to_ptr = args[1] as usize;
        let value_lo = args[2] as u64;
        let value_hi = args[3] as u64;
        let value = value_lo | (value_hi << 32);

        if matches!(
            metering.on_syscall_data(SYSCALL_TRANSFER, 20),
            MeterResult::Halt
        ) {
            panic!("Metering halted SYSCALL_TRANSFER");
        }

        let borrowed = memory.as_ref();
        let (to_start, to_end) = va_range(to_ptr, 20);
        let to_slice = borrowed.mem_slice(to_start, to_end).expect("invalid to ptr");

        let mut _to = [0u8; 20];
        _to.copy_from_slice(to_slice.as_ref());
        let _ = value;
        // Host integration removed; transfer is currently unsupported.
        1
    }

    fn sys_balance(
        &mut self,
        args: [u32; 6],
        memory: Memory,
        metering: &mut dyn Metering,
    ) -> u32 {
        // args: a1 = address pointer (20 bytes)
        let addr_ptr = args[0] as usize;
        if matches!(
            metering.on_syscall_data(SYSCALL_BALANCE, 20),
            MeterResult::Halt
        ) {
            panic!("Metering halted SYSCALL_BALANCE");
        }
        let addr = {
            let borrowed = memory.as_ref();
            let (addr_start, addr_end) = va_range(addr_ptr, 20);
            let addr_slice = borrowed
                .mem_slice(addr_start, addr_end)
                .expect("invalid addr ptr");
            let mut addr = [0u8; 20];
            addr.copy_from_slice(addr_slice.as_ref());
            addr
        };

        let bal = self
            .state
            .borrow()
            .get_account(&Address(addr))
            .map(|acc| acc.balance)
            .unwrap_or(0);
        match self.alloc_on_heap(&memory, &bal.to_le_bytes(), 8) {
            Some(ptr) => ptr.as_u32(),
            None => 0,
        }
    }

    /// Minimal `brk(2)` implementation:
    /// - a0 = new_break; if 0, return current break.
    /// - Only moves the break forward; shrink requests are ignored.
    fn sys_brk(&mut self, args: [u32; 6], memory: Memory, _metering: &mut dyn Metering) -> u32 {
        let new_brk = args[0];
        let current = self.ensure_heap_ptr();
        if new_brk == 0 {
            return current;
        }
        if new_brk >= current {
            self.set_heap_ptr(new_brk);
            new_brk
        } else {
            current
        }
    }

}

fn va_range(ptr: usize, len: usize) -> (VirtualAddress, VirtualAddress) {
    let start = VirtualAddress(ptr as u32);
    let end = start.wrapping_add(len as u32);
    (start, end)
}
