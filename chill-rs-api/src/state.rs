use crate::error::ChillApiError;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    borsh::try_from_slice_unchecked,
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

impl StateType {
    pub const LEN: usize = 1;
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

impl Fees {
    pub const LEN: usize = 8 * 5;
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Recipient {
    pub address: Pubkey,
    pub mint_share: u8,
    pub transaction_share: u8,
}

impl Recipient {
    pub const LEN: usize = 32 + 1 + 1;
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Config {
    state_type: StateType,
    pub mint: Pubkey,
    pub fees: Fees,
    pub recipients: Vec<Recipient>,
}

impl Sealed for Config {}

impl IsInitialized for Config {
    fn is_initialized(&self) -> bool {
        self.state_type == StateType::Config
    }
}

impl Pack for Config {
    const LEN: usize = StateType::LEN
        + 32
        + Fees::LEN
        + Self::VECTOR_PREFIX
        + Self::MAX_RECIPIENT_NUMBER * Recipient::LEN;

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_unchecked(src).map_err(|e| e.into())
    }
}

impl Config {
    const VECTOR_PREFIX: usize = 4;

    pub const MAX_RECIPIENT_NUMBER: usize = 3;

    pub fn new(
        mint: &Pubkey,
        fees: Fees,
        recipients: Vec<Recipient>,
    ) -> Result<Self, ProgramError> {
        if recipients.len() > Self::MAX_RECIPIENT_NUMBER {
            return Err(ChillApiError::MaximumRecipientsNumberExceeded.into());
        }

        if !recipients.is_empty() {
            let mint_share_sum = recipients.iter().map(|r| r.mint_share).sum::<u8>();
            let transaction_share_sum = recipients.iter().map(|r| r.mint_share).sum::<u8>();
            if mint_share_sum != 100 || transaction_share_sum != 100 {
                return Err(ChillApiError::InvalidShares.into());
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

#[cfg(test)]
mod tests {
    use solana_program::borsh::try_from_slice_unchecked;
    use solana_sdk::{signature::Keypair, signer::Signer};

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
                address: Keypair::new().pubkey(),
                mint_share: share,
                transaction_share: share,
            };

            assert_eq!(recipient.try_to_vec().unwrap().len(), Recipient::LEN);

            recipients.push(recipient);
            remaining_share -= share;
        }

        let last_recipient = Recipient {
            address: Keypair::new().pubkey(),
            mint_share: remaining_share,
            transaction_share: remaining_share,
        };

        recipients.push(last_recipient);
        recipients
    }

    #[test]
    fn config() {
        let mint = Keypair::new().pubkey();
        let fees = Fees::default();

        assert_eq!(fees.try_to_vec().unwrap().len(), Fees::LEN);

        for i in 0..=Config::MAX_RECIPIENT_NUMBER {
            let mut recipients = get_recipients(i as u8);
            let config = Config::new(&mint, fees.clone(), recipients.clone()).unwrap();

            let mut buffer = [0; Config::LEN];
            config.serialize(&mut buffer.as_mut()).unwrap();

            let unpacked_config = try_from_slice_unchecked(&buffer).unwrap();
            assert_eq!(config, unpacked_config);

            if i == Config::MAX_RECIPIENT_NUMBER {
                assert_eq!(config.try_to_vec().unwrap().len(), Config::LEN);
            } else {
                assert!(config.try_to_vec().unwrap().len() <= Config::LEN);
            }

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
