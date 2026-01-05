#![no_std]
#![no_main]

extern crate clibc;
use clibc::{
    DataParser, Map, StorageKey, entrypoint, event, fire_event, logf, persist_struct,
    require, router::route, types::{address::Address, o::O, result::Result}, vm_panic,
};

// Persistent structs
persist_struct!(Metadata {
    total_supply: u32,
    decimals: u8,
});

event!(Minted {
    caller => Address,
    amount => u32,
});

event!(Transfer {
    from => Address,
    to => Address,
    value => u32,
});

Map!(Balances);
Map!(Allowances);

struct AllowanceKey {
    bytes: [u8; 40],
}

impl AllowanceKey {
    fn new(owner: Address, spender: Address) -> Self {
        let mut bytes = [0u8; 40];
        bytes[..20].copy_from_slice(&owner.0);
        bytes[20..].copy_from_slice(&spender.0);
        Self { bytes }
    }
}

impl StorageKey for AllowanceKey {
    fn as_storage_key(&self) -> &[u8] {
        &self.bytes
    }
}

unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {
    route(data, program, caller, |_to, _from, call| {
        match call.selector {
            0x01 => {
                init(&program, caller, call.args);
                Result::new(true, 0)
            }
            0x02 => {
                let mut parser = DataParser::new(call.args);
                let to = parser.read_address();
                let amount = parser.read_u32();
                transfer(&program, caller, to, amount);
                Result::new(true, 0)
            }
            0x03 => {
                let mut parser = DataParser::new(call.args);
                let spender = parser.read_address();
                let amount = parser.read_u32();
                approve(&program, caller, spender, amount);
                Result::new(true, 0)
            }
            0x04 => {
                let mut parser = DataParser::new(call.args);
                let from = parser.read_address();
                let to = parser.read_address();
                let amount = parser.read_u32();
                transfer_from(&program, caller, from, to, amount);
                Result::new(true, 0)
            }
            0x05 => {
                let mut parser = DataParser::new(call.args);
                let owner = parser.read_address();
                let b = balance_of(&program, owner);
                Result::with_u32(b)
            }
            _ => vm_panic(b"unknown selector"),
        }
    })
}

fn init(program: &Address, caller: Address, args: &[u8]) {
    logf!("init called");
    let mut meta = match Metadata::load(program) {
        O::Some(_) => vm_panic(b"already initialized"),
        O::None => Metadata {
            total_supply: 0,
            decimals: 0,
        },
    };

    logf!("initializing");

    let mut parser = DataParser::new(args);
    let total_supply = parser.read_u32();
    let decimals = parser.read_bytes(1)[0];

    logf!("total supply: %d", total_supply);
    logf!("decimals: %d", decimals);

    meta.total_supply = total_supply;
    meta.decimals = decimals;
    meta.store(program);

    // mint to caller
    mint(program, caller, total_supply);
}

fn mint(program: &Address, caller: Address, val: u32) {
    logf!("minting: %d tokens", val);
    fire_event!(Minted::new(caller, val));
    Balances::set(program, caller, val);
}

fn transfer(program: &Address, caller: Address, to: Address, amount: u32) {
    logf!("erc20: transfer amount=%d", amount);
    let from_bal = match Balances::get(program, caller) {
        O::Some(bal) => bal,
        O::None => 0,
    };

    if from_bal < amount {
        vm_panic(b"insufficient");
    }

    let to_bal = match Balances::get(program, to) {
        O::Some(bal) => bal,
        O::None => 0,
    };

    Balances::set(program, caller, from_bal - amount);
    Balances::set(program, to, to_bal + amount);

    fire_event!(Transfer::new(caller, to, amount));
}

fn approve(program: &Address, caller: Address, spender: Address, amount: u32) {
    let key = AllowanceKey::new(caller, spender);
    Allowances::set(program, key, amount);
}

fn transfer_from(
    program: &Address,
    caller: Address,
    from: Address,
    to: Address,
    amount: u32,
) {
    let allowance = match Allowances::get(program, AllowanceKey::new(from, caller)) {
        O::Some(val) => val,
        O::None => 0,
    };
    require(allowance >= amount, b"allowance insufficient");

    let from_bal = match Balances::get(program, from) {
        O::Some(bal) => bal,
        O::None => 0,
    };
    if from_bal < amount {
        vm_panic(b"insufficient");
    }

    let to_bal = match Balances::get(program, to) {
        O::Some(bal) => bal,
        O::None => 0,
    };

    Allowances::set(program, AllowanceKey::new(from, caller), allowance - amount);
    Balances::set(program, from, from_bal - amount);
    Balances::set(program, to, to_bal + amount);

    fire_event!(Transfer::new(from, to, amount));
}

fn balance_of(program: &Address, owner: Address) -> u32 {
    match Balances::get(program, owner) {
        O::Some(bal) => bal,
        O::None => 0,
    }
}
// ---- Entry point ----
entrypoint!(main_entry);
