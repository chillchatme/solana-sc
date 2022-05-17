use crate::state::SEC_PER_DAY;
use anchor_lang::prelude::*;

pub fn current_day() -> Result<u64> {
    let clock = Clock::get()?;
    let timestamp = clock.unix_timestamp as u64;
    Ok(timestamp.checked_div(SEC_PER_DAY).unwrap())
}
