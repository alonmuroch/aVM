#[path = "common/utils.rs"]
mod utils;

#[path = "common/test_runner.rs"]
mod test_runner;

#[path = "common/state.rs"]
mod state_helper;

#[path = "common/ecdsa.rs"]
mod ecdsa;

#[path = "common/router.rs"]
mod router;

use router::{HostFuncCall, encode_router_calls};
use types::transaction::{Transaction, TransactionBundle, TransactionType};
use compiler::EventAbi;
pub use ecdsa::{ECDSA_HASH, ECDSA_SK_BYTES, build_ecdsa_payload};
use once_cell::sync::Lazy;
pub use test_runner::TestRunner;
use utils::{get_program_code, load_abi_from_file, load_abis_from_files, to_address};

/// Centralized ELF binary paths for testing
pub struct ElfBinary {
    pub name: &'static str,
    pub path: &'static str,
    pub description: &'static str,
}

/// All ELF binaries used in tests
pub const ELF_BINARIES: &[ElfBinary] = &[
    ElfBinary {
        name: "simple",
        path: "bin/simple",
        description: "Simple test program",
    },
    ElfBinary {
        name: "multi_func",
        path: "bin/multi_func",
        description: "Multiple function test",
    },
    ElfBinary {
        name: "logging",
        path: "bin/logging",
        description: "Logging functionality test",
    },
    ElfBinary {
        name: "storage",
        path: "bin/storage",
        description: "Storage operations test",
    },
    ElfBinary {
        name: "call_program",
        path: "bin/call_program",
        description: "Program calling test",
    },
    ElfBinary {
        name: "erc20",
        path: "bin/erc20",
        description: "ERC20 token contract",
    },
    ElfBinary {
        name: "lib_import",
        path: "bin/lib_import",
        description: "Library import test",
    },
    ElfBinary {
        name: "allocator_demo",
        path: "bin/allocator_demo",
        description: "Memory allocator demonstration",
    },
    ElfBinary {
        name: "native_transfer",
        path: "bin/native_transfer",
        description: "Native token transfer via syscall",
    },
    ElfBinary {
        name: "dex",
        path: "bin/dex",
        description: "Simple AMM between AM and ERC20",
    },
];

/// Get an ELF binary by name
pub fn get_elf_by_name(name: &str) -> Option<&'static ElfBinary> {
    ELF_BINARIES.iter().find(|elf| elf.name == name)
}

/// Get the full path for an ELF binary
pub fn get_elf_path(name: &str) -> Option<String> {
    get_elf_by_name(name).map(|elf| format!("crates/examples/{}", elf.path))
}

#[derive(Debug)]
pub struct TestCase<'a> {
    pub name: &'a str,
    pub expected_success: bool,
    pub expected_error_code: u32,
    pub expected_data: Option<Vec<u8>>,
    pub bundle: TransactionBundle,
    pub abi: Option<Vec<EventAbi>>,
    pub address_mappings: Vec<(&'a str, &'a str)>, // (address, binary_name)
}

pub static TEST_CASES: Lazy<Vec<TestCase<'static>>> = Lazy::new(|| {
    vec![
        TestCase {
            name: "erc20",
            expected_success: true,
            expected_error_code: 0,
            expected_data: Some(vec![128, 240, 250, 2]), // Expected data: 50,000,000 in little-endian
            abi: load_abi_from_file("bin/erc20.abi.json"),
            address_mappings: vec![("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1", "erc20")],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1"),
                    data: get_program_code("erc20"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: encode_router_calls(&[HostFuncCall {
                        selector: 0x01, // initialize
                        args: (|| {
                            // max supply
                            let max_supply: u32 = 100000000; // 100 million
                            let mut max_supply_bytes: Vec<u8> = max_supply.to_le_bytes().to_vec();

                            // decimals
                            let decimals: u8 = 18;

                            // combine
                            max_supply_bytes.extend(vec![decimals]);
                            max_supply_bytes
                        })(),
                    }]),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: encode_router_calls(&[HostFuncCall {
                        selector: 0x02, // transfer
                        args: (|| {
                            // to address (20 bytes)
                            let to_addr = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d2");
                            let mut args = to_addr.0.to_vec();

                            // amount (4 bytes)
                            let amount: u32 = 50000000; // 50 million tokens
                            args.extend(amount.to_le_bytes());

                            args
                        })(),
                    }]),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: encode_router_calls(&[HostFuncCall {
                        selector: 0x05, // balance_of
                        args: (|| {
                            // check balance of the original caller (d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0)
                            let owner_addr = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0");
                            owner_addr.0.to_vec()
                        })(),
                    }]),
                    value: 0,
                    nonce: 0,
                },
            ]),
        },
        TestCase {
            name: "call program",
            expected_success: true,
            expected_error_code: 0,
            expected_data: Some(vec![100, 0, 0, 0]), // Expected data: 100 in little-endian
            abi: None,
            address_mappings: vec![
                ("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0", "call_program"),
                ("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1", "simple"),
            ],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: get_program_code("call_program"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1"),
                    data: get_program_code("simple"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: (|| {
                        let mut data = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1")
                            .0
                            .to_vec();
                        data.extend(vec![100, 0, 0, 0, 42, 0, 0, 0]);
                        data
                    })(),
                    value: 0,
                    nonce: 0,
                },
            ]),
        },
        TestCase {
            name: "account create (storage)",
            expected_success: true,
            expected_error_code: 0,
            expected_data: None,
            abi: None,
            address_mappings: vec![("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0", "storage")],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: get_program_code("storage"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: vec![],
                    value: 0,
                    nonce: 0,
                },
            ]),
        },
        TestCase {
            name: "account create (simple)",
            expected_success: true,
            expected_error_code: 0,
            expected_data: Some(vec![100, 0, 0, 0]), // Expected data: 100 in little-endian
            abi: None,
            address_mappings: vec![("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0", "simple")],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: get_program_code("simple"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: vec![
                        100, 0, 0, 0, // first u64 = 100
                        42, 0, 0, 0, // second u64 = 42
                    ],
                    value: 0,
                    nonce: 0,
                },
            ]),
        },
        TestCase {
            name: "multi function (simple)",
            expected_success: true,
            expected_error_code: 0,
            expected_data: Some(vec![100, 0, 0, 0]), // Expected data: 100 in little-endian
            abi: None,
            address_mappings: vec![("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0", "multi_func")],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: get_program_code("multi_func"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: encode_router_calls(&[HostFuncCall {
                        selector: 0x01,
                        args: vec![
                            100, 0, 0, 0, // first = 100
                            42, 0, 0, 0, // second = 42
                        ],
                    }]),
                    value: 0,
                    nonce: 0,
                },
            ]),
        },
        TestCase {
            name: "allocator demo",
            expected_success: true,
            expected_error_code: 0,
            expected_data: None, //Some(b"VM allocator demo completed successfully!".to_vec()),
            abi: None,
            address_mappings: vec![("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0", "allocator_demo")],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: get_program_code("allocator_demo"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    // 6 x u32 little-endian:
                    // Vec: 12, 15, 100; Map: 95, 87, 92
                    data: vec![
                        12, 0, 0, 0, 15, 0, 0, 0, 100, 0, 0, 0, 95, 0, 0, 0, 87, 0, 0, 0, 92, 0, 0,
                        0,
                    ],
                    value: 0,
                    nonce: 0,
                },
            ]),
        },
        TestCase {
            name: "native transfer",
            expected_success: true,
            expected_error_code: 0,
            expected_data: None,
            abi: None,
            address_mappings: vec![],
            bundle: TransactionBundle::new(vec![Transaction {
                tx_type: TransactionType::Transfer,
                to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                data: vec![],
                value: 10,
                nonce: 0,
            }]),
        },
        TestCase {
            name: "guest transfer syscall",
            expected_success: true,
            expected_error_code: 0,
            expected_data: Some({
                let mut v = 42u128.to_le_bytes().to_vec();
                v
            }),
            abi: None,
            address_mappings: vec![(
                "d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d4",
                "native_transfer",
            )],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d4"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                    data: get_program_code("native_transfer"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d4"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                    data: (|| {
                        let mut data = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0")
                            .0
                            .to_vec();
                        data.extend_from_slice(&42u64.to_le_bytes());
                        data
                    })(),
                    value: 0,
                    nonce: 1,
                },
            ]),
        },
        TestCase {
            name: "dex amm",
            expected_success: true,
            expected_error_code: 0,
            expected_data: Some({
                let mut buf = Vec::new();
                buf.extend_from_slice(&101000u128.to_le_bytes());
                buf.extend_from_slice(&495050u128.to_le_bytes());
                buf
            }),
            abi: load_abis_from_files(&["bin/erc20.abi.json", "bin/dex.abi.json"]),
            address_mappings: vec![
                ("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1", "erc20"),
                ("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d5", "dex"),
            ],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                    data: get_program_code("erc20"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                    data: encode_router_calls(&[HostFuncCall {
                        selector: 0x01, // init
                        args: (|| {
                            let mut args = Vec::new();
                            let supply: u32 = 1_000_000;
                            args.extend_from_slice(&supply.to_le_bytes());
                            args.push(0); // decimals
                            args
                        })(),
                    }]),
                    value: 0,
                    nonce: 1,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                    data: encode_router_calls(&[HostFuncCall {
                        selector: 0x03, // approve
                        args: (|| {
                            let mut args = to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d5")
                                .0
                                .to_vec();
                            let amount: u32 = 500_000;
                            args.extend_from_slice(&amount.to_le_bytes());
                            args
                        })(),
                    }]),
                    value: 0,
                    nonce: 2,
                },
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d5"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                    data: get_program_code("dex"),
                    value: 0,
                    nonce: 3,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d5"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                    data: {
                        let mut data = Vec::new();
                        data.push(0x01); // add liquidity
                        data.extend_from_slice(&100_000u64.to_le_bytes()); // AM in
                        data.extend_from_slice(&500_000u64.to_le_bytes()); // token target
                        data
                    },
                    value: 0,
                    nonce: 4,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d5"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d2"),
                    data: {
                        let mut data = Vec::new();
                        data.push(0x03); // swap
                        data.push(0x00); // am -> token
                        data.extend_from_slice(&1_000u64.to_le_bytes());
                        data
                    },
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d5"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d3"),
                    data: {
                        let mut data = Vec::new();
                        data.push(0x02); // remove liquidity
                        data.extend_from_slice(&100_000u64.to_le_bytes());
                        data
                    },
                    value: 0,
                    nonce: 5,
                },
            ]),
        },
        TestCase {
            name: "ecdsa verify",
            expected_success: true,
            expected_error_code: 0,
            expected_data: None,
            abi: None,
            address_mappings: vec![("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0", "ecdsa_verify")],
            bundle: TransactionBundle::new(vec![
                Transaction {
                    tx_type: TransactionType::CreateAccount,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: get_program_code("ecdsa_verify"),
                    value: 0,
                    nonce: 0,
                },
                Transaction {
                    tx_type: TransactionType::ProgramCall,
                    to: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    from: to_address("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d0"),
                    data: build_ecdsa_payload(),
                    value: 0,
                    nonce: 1,
                },
            ]),
        },
    ]
});

#[test]
fn test_examples() {
    TestRunner::default().execute().unwrap()
}
