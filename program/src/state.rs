#![allow(clippy::ptr_offset_with_cast)]

use crate::error::ChillError;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum StateType {
    Uninitialized,
    Config,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Fees {
    pub character: u64,
    pub pet: u64,
    pub emote: u64,
    pub tileset: u64,
    pub item: u64,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Recipient {
    pub address: Pubkey,
    pub mint_share: u8,
    pub transaction_share: u8,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    state_type: StateType,
    pub mint: Pubkey,
    pub fees: Fees,
    pub recipients: Vec<Recipient>,
}

impl Sealed for Config {}

impl Pack for Config {
    const LEN: usize = 175;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Config::LEN];
        let (state_type, mint, fees, recipients_array) =
            mut_array_refs![dst, 1, 32, 40, Config::MAX_RECIPIENT_NUMBER * 34];

        state_type.copy_from_slice(&self.state_type.try_to_vec().unwrap());
        mint.copy_from_slice(&self.mint.try_to_vec().unwrap());
        fees.copy_from_slice(&self.fees.try_to_vec().unwrap());

        for (i, recipient) in self.recipients.iter().enumerate() {
            let dst = array_mut_ref![recipients_array, i * 34, 34];
            dst.copy_from_slice(&recipient.try_to_vec().unwrap())
        }
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Config::LEN];
        let (state_type, mint, fees, recipients_array) =
            array_refs![src, 1, 32, 40, Config::MAX_RECIPIENT_NUMBER * 34];

        let state_type = StateType::try_from_slice(state_type)?;
        if state_type != StateType::Config {
            return Err(ProgramError::InvalidAccountData);
        }

        let mint = Pubkey::try_from_slice(mint)?;
        let fees = Fees::try_from_slice(fees)?;

        let mut recipients = Vec::with_capacity(Config::MAX_RECIPIENT_NUMBER);
        let zero_pubkey = Pubkey::new_from_array([0; 32]);

        for i in 0..Config::MAX_RECIPIENT_NUMBER {
            let recipient_data = array_ref![recipients_array, i * 34, 34];
            let recipient = Recipient::try_from_slice(recipient_data)?;
            if recipient.address != zero_pubkey || recipient.mint_share != 0 {
                recipients.push(recipient);
            }
        }

        Ok(Self {
            state_type,
            mint,
            fees,
            recipients,
        })
    }
}

impl IsInitialized for Config {
    fn is_initialized(&self) -> bool {
        self.state_type == StateType::Config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            state_type: StateType::Config,
            mint: Pubkey::default(),
            fees: Fees::default(),
            recipients: Vec::default(),
        }
    }
}

impl Config {
    pub const MAX_RECIPIENT_NUMBER: usize = 3;

    pub fn new(
        mint: &Pubkey,
        fees: Fees,
        recipients: Vec<Recipient>,
    ) -> Result<Self, ProgramError> {
        if recipients.len() > Self::MAX_RECIPIENT_NUMBER {
            return Err(ChillError::MaximumRecipientsNumberExceeded.into());
        }

        if !recipients.is_empty() {
            let mint_share_sum = recipients.iter().map(|r| r.mint_share).sum::<u8>();
            let transaction_share_sum = recipients.iter().map(|r| r.mint_share).sum::<u8>();
            if mint_share_sum != 100 || transaction_share_sum != 100 {
                return Err(ChillError::InvalidShares.into());
            }
        }

        Ok(Self {
            state_type: StateType::Config,
            mint: *mint,
            fees,
            recipients,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_recipients(number: u8) -> Vec<Recipient> {
        if number == 0 {
            return Vec::new();
        }

        let mut remaining_share = 100;
        let share = 100 / number - 1;

        let mut recipients = Vec::with_capacity(number.into());
        for _ in 1..number {
            let recipient = Recipient {
                address: Pubkey::new_unique(),
                mint_share: share,
                transaction_share: share,
            };

            recipients.push(recipient);
            remaining_share -= share;
        }

        let last_recipient = Recipient {
            address: Pubkey::new_unique(),
            mint_share: remaining_share,
            transaction_share: remaining_share,
        };

        recipients.push(last_recipient);
        recipients
    }

    #[test]
    fn config() {
        let mint = Pubkey::new_unique();
        let fees = Fees::default();

        for i in 0..Config::MAX_RECIPIENT_NUMBER {
            let mut recipients = get_recipients(i as u8);
            let config = Config::new(&mint, fees.clone(), recipients.clone()).unwrap();

            let mut buffer = [0; Config::LEN];
            Config::pack(config.clone(), &mut buffer).unwrap();

            let unpacked_config = Config::unpack(&buffer).unwrap();
            assert_eq!(config, unpacked_config);

            if !recipients.is_empty() {
                recipients[0].mint_share -= 1;
                let config_wrong_share = Config::new(&mint, fees.clone(), recipients);
                assert!(config_wrong_share.is_err());
            }
        }

        let recipients = get_recipients(Config::MAX_RECIPIENT_NUMBER as u8 + 1);
        let config_overflow = Config::new(&mint, fees.clone(), recipients);
        assert!(config_overflow.is_err());
    }
}
