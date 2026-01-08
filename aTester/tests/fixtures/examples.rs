use compiler::elf::parse_elf_from_bytes;
use types::address::Address;
use types::transaction::{Transaction, TransactionBundle, TransactionType};

pub struct ExpectedResult {
    pub success: bool,
    pub error_code: u32,
    pub data: Vec<u8>,
}

pub struct ExampleCase {
    pub name: &'static str,
    pub description: &'static str,
    pub bundle: TransactionBundle,
}

pub fn test_state_bytes() -> Vec<u8> {
    let mut state = state::State::new();
    for addr_hex in [
        "d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d2",
        "d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3",
    ] {
        let addr = to_address(addr_hex);
        let account = state.get_account_mut(&addr);
        account.balance = 1_000_000_000u128;
    }
    state.encode()
}

pub fn all_example_cases() -> Result<Vec<ExampleCase>, String> {
    Ok(vec![
        ExampleCase {
            name: "erc20",
            description: "ERC-20 init, transfer, and balance query flow",
            bundle: build_erc20_bundle()?,
        },
        ExampleCase {
            name: "call program",
            description: "Cross-contract call with nested program execution",
            bundle: build_call_program_bundle()?,
        },
        ExampleCase {
            name: "account create (storage)",
            description: "Create a contract and invoke a storage call",
            bundle: build_account_create_storage_bundle()?,
        },
        ExampleCase {
            name: "account create (simple)",
            description: "Create a simple contract and verify return data",
            bundle: build_account_create_simple_bundle()?,
        },
        ExampleCase {
            name: "multi function (simple)",
            description: "Router-style call into a multi-function contract",
            bundle: build_multi_function_simple_bundle()?,
        },
        ExampleCase {
            name: "allocator demo",
            description: "Heap allocation and collection usage in guest code",
            bundle: build_allocator_demo_bundle()?,
        },
        ExampleCase {
            name: "native transfer",
            description: "Native value transfer without a contract call",
            bundle: build_native_transfer_bundle(),
        },
        ExampleCase {
            name: "guest transfer syscall",
            description: "Program issues a native transfer syscall",
            bundle: build_guest_transfer_syscall_bundle()?,
        },
        ExampleCase {
            name: "dex amm",
            description: "AMM lifecycle: init, approve, add/remove liquidity, swap",
            bundle: build_dex_amm_bundle()?,
        },
        ExampleCase {
            name: "ecdsa verify",
            description: "ECDSA signature verification within the VM",
            bundle: build_ecdsa_verify_bundle()?,
        },
    ])
}

pub fn expected_for(name: &str) -> Option<ExpectedResult> {
    match name {
        "erc20" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: vec![128, 240, 250, 2],
        }),
        "call program" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: vec![100, 0, 0, 0],
        }),
        "account create (storage)" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: Vec::new(),
        }),
        "account create (simple)" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: vec![100, 0, 0, 0],
        }),
        "multi function (simple)" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: vec![100, 0, 0, 0],
        }),
        "allocator demo" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: Vec::new(),
        }),
        "native transfer" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: Vec::new(),
        }),
        "guest transfer syscall" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: 42u128.to_le_bytes().to_vec(),
        }),
        "dex amm" => {
            let mut buf = Vec::new();
            buf.extend_from_slice(&101000u128.to_le_bytes());
            buf.extend_from_slice(&495050u128.to_le_bytes());
            Some(ExpectedResult {
                success: true,
                error_code: 0,
                data: buf,
            })
        }
        "ecdsa verify" => Some(ExpectedResult {
            success: true,
            error_code: 0,
            data: Vec::new(),
        }),
        _ => None,
    }
}

struct HostFuncCall {
    selector: u8,
    args: Vec<u8>,
}

fn encode_router_calls(calls: &[HostFuncCall]) -> Vec<u8> {
    let mut encoded = Vec::new();
    for call in calls {
        let len = call.args.len();
        assert!(len <= 255, "argument too long for 1-byte length field");
        encoded.push(call.selector);
        encoded.push(len as u8);
        encoded.extend_from_slice(&call.args);
    }
    encoded
}

fn build_erc20_bundle() -> Result<TransactionBundle, String> {
    let deployer = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
    let contract = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            from: deployer,
            to: contract,
            data: get_program_code("erc20")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: contract,
            from: deployer,
            data: encode_router_calls(&[HostFuncCall {
                selector: 0x01,
                args: {
                    let max_supply: u32 = 100000000;
                    let mut max_supply_bytes = max_supply.to_le_bytes().to_vec();
                    max_supply_bytes.extend(vec![18u8]);
                    max_supply_bytes
                },
            }]),
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: contract,
            from: deployer,
            data: encode_router_calls(&[HostFuncCall {
                selector: 0x02,
                args: {
                    let to_addr = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d2");
                    let mut args = to_addr.0.to_vec();
                    let amount: u32 = 50000000;
                    args.extend(amount.to_le_bytes());
                    args
                },
            }]),
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: contract,
            from: deployer,
            data: encode_router_calls(&[HostFuncCall {
                selector: 0x05,
                args: {
                    let owner = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
                    owner.0.to_vec()
                },
            }]),
            value: 0,
            nonce: 0,
        },
    ]))
}

fn build_call_program_bundle() -> Result<TransactionBundle, String> {
    let caller = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
    let callee = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            from: caller,
            to: caller,
            data: get_program_code("call_program")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::CreateAccount,
            from: caller,
            to: callee,
            data: get_program_code("simple")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: caller,
            from: caller,
            data: {
                let mut data = callee.0.to_vec();
                data.extend(vec![100, 0, 0, 0, 42, 0, 0, 0]);
                data
            },
            value: 0,
            nonce: 0,
        },
    ]))
}

fn build_account_create_storage_bundle() -> Result<TransactionBundle, String> {
    let addr = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            to: addr,
            from: addr,
            data: get_program_code("storage")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: addr,
            from: addr,
            data: vec![],
            value: 0,
            nonce: 0,
        },
    ]))
}

fn build_account_create_simple_bundle() -> Result<TransactionBundle, String> {
    let addr = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            to: addr,
            from: addr,
            data: get_program_code("simple")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: addr,
            from: addr,
            data: vec![100, 0, 0, 0, 42, 0, 0, 0],
            value: 0,
            nonce: 0,
        },
    ]))
}

fn build_multi_function_simple_bundle() -> Result<TransactionBundle, String> {
    let addr = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            to: addr,
            from: addr,
            data: get_program_code("multi_func")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: addr,
            from: addr,
            data: encode_router_calls(&[HostFuncCall {
                selector: 0x01,
                args: vec![100, 0, 0, 0, 42, 0, 0, 0],
            }]),
            value: 0,
            nonce: 0,
        },
    ]))
}

fn build_allocator_demo_bundle() -> Result<TransactionBundle, String> {
    let addr = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            to: addr,
            from: addr,
            data: get_program_code("allocator_demo")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: addr,
            from: addr,
            data: vec![
                12, 0, 0, 0, 15, 0, 0, 0, 100, 0, 0, 0, 95, 0, 0, 0, 87, 0, 0, 0, 92, 0, 0, 0,
            ],
            value: 0,
            nonce: 0,
        },
    ]))
}

fn build_native_transfer_bundle() -> TransactionBundle {
    TransactionBundle::new(vec![Transaction {
        tx_type: TransactionType::Transfer,
        to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
        from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
        data: vec![],
        value: 10,
        nonce: 0,
    }])
}

fn build_guest_transfer_syscall_bundle() -> Result<TransactionBundle, String> {
    let program = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d4");
    let sender = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3");
    let recipient = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            to: program,
            from: sender,
            data: get_program_code("native_transfer")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: program,
            from: sender,
            data: {
                let mut data = recipient.0.to_vec();
                data.extend_from_slice(&42u64.to_le_bytes());
                data
            },
            value: 0,
            nonce: 1,
        },
    ]))
}

fn build_dex_amm_bundle() -> Result<TransactionBundle, String> {
    let erc20 = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1");
    let dex = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d5");
    let user2 = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d2");
    let user3 = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            to: erc20,
            from: user3,
            data: get_program_code("erc20")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: erc20,
            from: user3,
            data: encode_router_calls(&[HostFuncCall {
                selector: 0x01,
                args: {
                    let mut args = Vec::new();
                    let supply: u32 = 1_000_000;
                    args.extend_from_slice(&supply.to_le_bytes());
                    args.push(0);
                    args
                },
            }]),
            value: 0,
            nonce: 1,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: erc20,
            from: user3,
            data: encode_router_calls(&[HostFuncCall {
                selector: 0x03,
                args: {
                    let mut args = dex.0.to_vec();
                    let amount: u32 = 500_000;
                    args.extend_from_slice(&amount.to_le_bytes());
                    args
                },
            }]),
            value: 0,
            nonce: 2,
        },
        Transaction {
            tx_type: TransactionType::CreateAccount,
            to: dex,
            from: user3,
            data: get_program_code("dex")?,
            value: 0,
            nonce: 3,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: dex,
            from: user3,
            data: {
                let mut data = Vec::new();
                data.push(0x01);
                data.extend_from_slice(&100_000u64.to_le_bytes());
                data.extend_from_slice(&500_000u64.to_le_bytes());
                data
            },
            value: 0,
            nonce: 4,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: dex,
            from: user2,
            data: {
                let mut data = Vec::new();
                data.push(0x03);
                data.push(0x00);
                data.extend_from_slice(&1_000u64.to_le_bytes());
                data
            },
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: dex,
            from: user3,
            data: {
                let mut data = Vec::new();
                data.push(0x02);
                data.extend_from_slice(&100_000u64.to_le_bytes());
                data
            },
            value: 0,
            nonce: 5,
        },
    ]))
}

fn build_ecdsa_verify_bundle() -> Result<TransactionBundle, String> {
    let addr = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
    Ok(TransactionBundle::new(vec![
        Transaction {
            tx_type: TransactionType::CreateAccount,
            to: addr,
            from: addr,
            data: get_program_code("ecdsa_verify")?,
            value: 0,
            nonce: 0,
        },
        Transaction {
            tx_type: TransactionType::ProgramCall,
            to: addr,
            from: addr,
            data: build_ecdsa_payload(),
            value: 0,
            nonce: 1,
        },
    ]))
}

fn build_ecdsa_payload() -> Vec<u8> {
    let mut payload =
        Vec::with_capacity(1 + ECDSA_PK_BYTES.len() + ECDSA_SIG_BYTES.len() + ECDSA_HASH.len());
    payload.push(ECDSA_PK_BYTES.len() as u8);
    payload.extend_from_slice(&ECDSA_PK_BYTES);
    payload.extend_from_slice(&ECDSA_SIG_BYTES);
    payload.extend_from_slice(&ECDSA_HASH);
    payload
}

const ECDSA_HASH: [u8; 32] = [
    0x3b, 0xbd, 0x38, 0x9e, 0x94, 0x1c, 0x63, 0x7f, 0x36, 0x32, 0xaa, 0xf4, 0x2f, 0x93, 0xb7, 0xb1,
    0xf1, 0x7c, 0x6f, 0x31, 0x86, 0x92, 0x01, 0x34, 0x1d, 0x5f, 0x28, 0x40, 0x61, 0x5c, 0xac, 0x2b,
];
const ECDSA_PK_BYTES: [u8; 33] = [
    0x02, 0xda, 0x8c, 0x8e, 0x0a, 0x4e, 0x5d, 0xfc, 0x76, 0x6f, 0xf1, 0xcb, 0xda, 0x27, 0x03, 0xea,
    0xcd, 0xb0, 0xdf, 0x07, 0xda, 0x19, 0xde, 0x65, 0x03, 0x51, 0x46, 0xdb, 0x9b, 0x9c, 0x8a, 0xb7,
    0x0c,
];
const ECDSA_SIG_BYTES: [u8; 64] = [
    0x13, 0xe3, 0x22, 0xb9, 0x33, 0x19, 0x17, 0x76, 0x6d, 0x8c, 0xbf, 0xe9, 0x9f, 0x1d, 0x44, 0xd8,
    0xeb, 0x4f, 0x1d, 0xb3, 0xca, 0xd1, 0x31, 0xaf, 0x92, 0xb2, 0xf2, 0x26, 0x3c, 0xe6, 0x60, 0x92,
    0x2a, 0x3a, 0xef, 0x94, 0xe6, 0x3e, 0x74, 0x06, 0xf4, 0x20, 0xee, 0x0c, 0x0c, 0xb6, 0x5f, 0xce,
    0xe0, 0x45, 0x26, 0xba, 0x9e, 0x36, 0xf6, 0x20, 0x92, 0x77, 0x73, 0x9d, 0x2d, 0x64, 0x37, 0xa2,
];

fn get_program_code(name: &str) -> Result<Vec<u8>, String> {
    let bytes = read_example_bin(name)?;
    let elf =
        parse_elf_from_bytes(&bytes).map_err(|e| format!("failed to parse elf for {name}: {e}"))?;

    let (code, code_start) = elf
        .get_flat_code()
        .ok_or_else(|| format!("no code section for {name}"))?;
    let (rodata, rodata_start) = elf.get_flat_rodata().unwrap_or((Vec::new(), u64::MAX));

    let mut total_len = code_start + code.len() as u64;
    if !rodata.is_empty() {
        total_len = rodata_start + rodata.len() as u64;
    }

    let mut combined = vec![0u8; total_len as usize];
    combined[code_start as usize..code_start as usize + code.len()].copy_from_slice(&code);
    if !rodata.is_empty() {
        combined[rodata_start as usize..rodata_start as usize + rodata.len()]
            .copy_from_slice(&rodata);
    }
    Ok(combined)
}

fn read_example_bin(name: &str) -> Result<Vec<u8>, String> {
    let base = workspace_root().join("crates/examples/bin");
    let mut candidates = vec![base.join(name), base.join(format!("{name}.elf"))];
    candidates.push(workspace_root().join("target/avm32/release").join(name));

    for path in candidates {
        if path.exists() {
            return std::fs::read(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()));
        }
    }
    Err(format!(
        "missing example binary for {name}; try running make -C crates/examples"
    ))
}

fn to_address(hex: &str) -> Address {
    assert!(hex.len() == 40, "hex string must be 40 characters");
    fn from_hex_char(c: u8) -> u8 {
        match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => panic!("invalid hex character"),
        }
    }
    let mut bytes = [0u8; 20];
    let hex_bytes = hex.as_bytes();
    for i in 0..20 {
        let hi = from_hex_char(hex_bytes[i * 2]);
        let lo = from_hex_char(hex_bytes[i * 2 + 1]);
        bytes[i] = (hi << 4) | lo;
    }
    Address(bytes)
}

fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map(std::path::PathBuf::from)
        .expect("missing workspace root")
}
