#![no_std]
#![no_main]

extern crate clibc;
use clibc::{
    DataParser, Map, entrypoint, event, fire_event, log, logf, persist_struct, require,
    router::route,
    types::{address::Address, o::O, result::Result},
    vm_panic,
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

unsafe fn main_entry(program: Address, caller: Address, data: &[u8]) -> Result {
    route(data, program, caller, |to, from, call| {
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
        O::Some(m) => vm_panic(b"already initialized"),
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

fn balance_of(program: &Address, owner: Address) -> u32 {
    match Balances::get(program, owner) {
        O::Some(bal) => bal,
        O::None => 0,
    }
}
// ---- Entry point ----
entrypoint!(main_entry);
