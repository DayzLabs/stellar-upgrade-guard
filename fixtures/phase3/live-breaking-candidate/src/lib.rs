#![no_std]

use soroban_sdk::{contract, contractimpl, Env, String, Symbol};

#[contract]
pub struct LiveBreakingCandidate;

#[contractimpl]
impl LiveBreakingCandidate {
    pub fn hello(env: Env, name: Symbol) -> String {
        let _ = name;
        String::from_str(&env, "hello")
    }
}
