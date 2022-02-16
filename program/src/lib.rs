pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

solana_program::declare_id!("64GbC4BYC6iSvrsoMtdYj7pTzLBUraCWQJMwX2srbVfk");
