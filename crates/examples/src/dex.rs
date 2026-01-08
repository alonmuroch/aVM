#![no_std]
#![no_main]

extern crate clibc;

use clibc::{
    DataParser, Map,
    call::call,
    entrypoint, event, fire_event, hex_address, persist_struct, require, transfer,
    types::{address::Address, o::O, result::Result},
    vm_panic,
};

// Generated ABI client for ERC20 (included like in call_program example)
include!("../bin/erc20_abi.rs");

// Persistent pool state
persist_struct!(Pool {
    reserve_am: u128,
    reserve_token: u128,
    total_liquidity: u128,
});

// Track liquidity shares per provider
Map!(Liquidity);

event!(LiquidityAdded {
    provider => Address,
    am_in => u64,
    token_in => u64,
});

event!(LiquidityRemoved {
    provider => Address,
    am_out => u64,
    token_out => u64,
});

event!(SwapExecuted {
    trader => Address,
    am_in => u64,
    token_out => u64,
});

// Use the existing ERC20 example program as the paired token.
fn erc20_address() -> Address {
    hex_address!("d5a3c7f85d2b6e91fa78cd3210b45f6ae913d0d1")
}

// Operation selectors
const ADD_LIQUIDITY: u8 = 0x01;
const REMOVE_LIQUIDITY: u8 = 0x02;
const SWAP: u8 = 0x03;

fn load_pool(program: &Address) -> Pool {
    match Pool::load(program) {
        O::Some(p) => p,
        O::None => Pool {
            reserve_am: 0,
            reserve_token: 0,
            total_liquidity: 0,
        },
    }
}

fn get_liquidity(program: &Address, owner: Address) -> u128 {
    match Liquidity::get(program, owner) {
        O::Some(v) => v,
        O::None => 0,
    }
}

fn add_liquidity(program: Address, caller: Address, mut parser: DataParser) -> Result {
    // Adds liquidity by pulling both legs (native AM and ERC20) from the caller,
    // mints LP shares proportional to the existing reserves, and emits an event.
    let erc20 = Erc20Contract::new(erc20_address());
    require(parser.remaining() >= 16, b"add: missing args");
    let am_in = parser.read_u64();
    let token_in = parser.read_u64() as u128;
    require(am_in > 0, b"add: zero am");
    require(token_in > 0, b"add: zero token");
    require(token_in <= u32::MAX as u128, b"add: token overflow");

    // Collect AM from caller into the pool address (native balance increases).
    require(transfer!(&program, am_in), b"add: am transfer failed");

    // Pull ERC20 from caller into the pool address.
    let ok = erc20
        .transfer_from(&program, caller, program, token_in as u32)
        .map(|r| r.success)
        .unwrap_or(false);
    require(ok, b"add: token transfer failed");

    let mut pool = load_pool(&program);

    let minted = if pool.total_liquidity == 0 {
        am_in as u128
    } else {
        require(
            pool.reserve_am > 0 && pool.reserve_token > 0,
            b"add: pool empty",
        );
        require(
            token_in * pool.reserve_am == (am_in as u128) * pool.reserve_token,
            b"add: ratio mismatch",
        );
        (am_in as u128 * pool.total_liquidity) / pool.reserve_am
    };

    pool.reserve_am = pool.reserve_am.saturating_add(am_in as u128);
    pool.reserve_token = pool.reserve_token.saturating_add(token_in);
    pool.total_liquidity = pool.total_liquidity.saturating_add(minted);
    pool.store(&program);

    let user_liq = get_liquidity(&program, caller).saturating_add(minted);
    Liquidity::set(&program, caller, user_liq);

    if token_in <= u64::MAX as u128 {
        fire_event!(LiquidityAdded::new(caller, am_in, token_in as u64));
    }

    let mut res = Result::new(true, 0);
    res.set_data(&minted.to_le_bytes());
    res
}

fn remove_liquidity(program: Address, caller: Address, mut parser: DataParser) -> Result {
    // Burns LP shares for AM + ERC20 payouts, updates reserves, and emits LiquidityRemoved.
    let erc20 = Erc20Contract::new(erc20_address());
    require(parser.remaining() >= 8, b"remove: missing args");
    let shares = parser.read_u64() as u128;
    require(shares > 0, b"remove: zero shares");

    let mut pool = load_pool(&program);
    let user_shares = get_liquidity(&program, caller);
    require(user_shares >= shares, b"remove: not enough shares");
    require(pool.total_liquidity > 0, b"remove: empty pool");

    // Compute pro-rata payouts.
    let am_out = (pool.reserve_am * shares) / pool.total_liquidity;
    let token_out = (pool.reserve_token * shares) / pool.total_liquidity;
    require(am_out <= u64::MAX as u128, b"remove: am overflow");

    pool.reserve_am = pool.reserve_am.saturating_sub(am_out);
    pool.reserve_token = pool.reserve_token.saturating_sub(token_out);
    pool.total_liquidity = pool.total_liquidity.saturating_sub(shares);
    pool.store(&program);

    Liquidity::set(&program, caller, user_shares - shares);

    // Pay out ERC20 tokens from pool balance.
    require(token_out <= u32::MAX as u128, b"remove: token overflow");
    let ok = erc20
        .transfer(&program, caller, token_out as u32)
        .map(|r| r.success)
        .unwrap_or(false);
    require(ok, b"remove: token transfer failed");

    // Pay native AM out to the provider. Note: with the current host interface,
    // native transfers debit the caller context.
    require(
        transfer!(&caller, am_out as u64),
        b"remove: am transfer failed",
    );

    fire_event!(LiquidityRemoved::new(
        caller,
        am_out as u64,
        token_out as u64
    ));

    // AM payouts are reported in the result for visibility.
    let mut res = Result::new(true, 0);
    let mut buf = [0u8; 32];
    buf[..16].copy_from_slice(&am_out.to_le_bytes());
    buf[16..32].copy_from_slice(&token_out.to_le_bytes());
    res.set_data(&buf);
    res
}

fn swap(program: Address, caller: Address, mut parser: DataParser) -> Result {
    // Constant-product swap. Direction 0 = AM -> ERC20, Direction 1 = ERC20 -> AM.
    let erc20 = Erc20Contract::new(erc20_address());
    require(parser.remaining() >= 9, b"swap: missing args");
    let direction = parser.read_bytes(1)[0];
    let amount = parser.read_u64();
    require(amount > 0, b"swap: zero amount");

    let mut pool = load_pool(&program);
    require(
        pool.reserve_am > 0 && pool.reserve_token > 0,
        b"swap: empty pool",
    );

    if direction == 0 {
        let am_in = amount;
        // Collect AM into the pool.
        let ok = transfer!(&program, am_in);
        require(ok, b"swap: am transfer failed");

        let token_out = (am_in as u128 * pool.reserve_token) / (pool.reserve_am + am_in as u128);
        require(token_out > 0, b"swap: zero output");
        require(
            token_out <= pool.reserve_token,
            b"swap: insufficient tokens",
        );
        require(token_out <= u32::MAX as u128, b"swap: token overflow");

        pool.reserve_am = pool.reserve_am.saturating_add(am_in as u128);
        pool.reserve_token = pool.reserve_token.saturating_sub(token_out);
        pool.store(&program);

        let ok = erc20
            .transfer(&program, caller, token_out as u32)
            .map(|r| r.success)
            .unwrap_or(false);
        require(ok, b"swap: token transfer failed");

        fire_event!(SwapExecuted::new(caller, am_in, token_out as u64));

        let mut res = Result::new(true, 0);
        res.set_data(&(token_out as u128).to_le_bytes());
        res
    } else {
        // ERC20 -> AM
        let token_in = amount as u128;
        require(token_in <= u32::MAX as u128, b"swap: token overflow");

        // Pull ERC20 into the pool.
        let ok = erc20
            .transfer_from(&program, caller, program, token_in as u32)
            .map(|r| r.success)
            .unwrap_or(false);
        require(ok, b"swap: token transfer failed");

        let am_out = (token_in * pool.reserve_am) / (pool.reserve_token + token_in);
        require(am_out > 0, b"swap: zero output");
        require(am_out <= pool.reserve_am, b"swap: insufficient am");
        require(am_out <= u64::MAX as u128, b"swap: am overflow");

        pool.reserve_token = pool.reserve_token.saturating_add(token_in);
        pool.reserve_am = pool.reserve_am.saturating_sub(am_out);
        pool.store(&program);

        // Pay native AM to the trader and leave ERC20 in the pool.
        require(transfer!(&caller, am_out as u64), b"swap: am payout failed");

        fire_event!(SwapExecuted::new(caller, am_out as u64, token_in as u64));

        let mut res = Result::new(true, 0);
        res.set_data(&(am_out as u128).to_le_bytes());
        res
    }
}

fn program_entry(program: Address, caller: Address, data: &[u8]) -> Result {
    // Simple selector-based router: first byte is op, remainder is args for the op handlers.
    if data.is_empty() {
        vm_panic(b"missing selector");
    }

    let mut parser = DataParser::new(data);
    let op = parser.read_bytes(1)[0];

    match op {
        ADD_LIQUIDITY => add_liquidity(program, caller, parser),
        REMOVE_LIQUIDITY => remove_liquidity(program, caller, parser),
        SWAP => swap(program, caller, parser),
        _ => vm_panic(b"unknown selector"),
    }
}

entrypoint!(program_entry);
