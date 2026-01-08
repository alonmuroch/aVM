#![no_std]
#![no_main]

extern crate clibc;
use clibc::persist_struct;
use clibc::types::address::Address;
use clibc::{entrypoint, require, types::result::Result};

// Struct 1: User profile
persist_struct!(User {
    id: u64,
    active: bool,
    level: u8,
});

// Struct 2: Config
persist_struct!(Config {
    retries: u64,
    timeout_ms: u64,
});

fn program_entry(program: Address, _caller: Address, _data: &[u8]) -> Result {
    // --- User ---
    require(User::load(&program).is_none(), b"user already exists");
    let mut user = User {
        id: 1000,
        active: false,
        level: 3,
    };
    user.level = 4;
    user.id = 40000;
    user.store(&program);

    // ... change local copy ...
    user.level = 5;
    user.id = 40001;
    require(user.level == 5, b"user local level must be 5");
    require(user.id == 40001, b"user local id must be 40001");

    let reloaded_user = User::load(&program).expect("user not found");
    require(reloaded_user.level == 4, b"user level must be 4");
    require(reloaded_user.id == 40000, b"user id must be 40000");

    // --- Config ---
    require(Config::load(&program).is_none(), b"config already exists");
    let mut config = Config {
        retries: 10,
        timeout_ms: 10,
    };
    config.retries = 13;
    config.timeout_ms = 100000;
    config.store(&program);

    // ... change local copy ...
    config.retries = 15;
    config.timeout_ms = 103000;
    require(config.retries == 15, b"config local retries must be 15");
    require(
        config.timeout_ms == 103000,
        b"config local timeout_ms must be 103000",
    );

    let reloaded_config = Config::load(&program).expect("config not found");
    require(reloaded_config.retries == 13, b"config retries must be 13");
    require(
        reloaded_config.timeout_ms == 100000,
        b"config timeout_ms must be 100000",
    );

    Result::new(true, 0)
}

entrypoint!(program_entry);
