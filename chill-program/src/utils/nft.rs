use crate::utils::TokenBuilder;
use chill_api::pda;
use mpl_token_metadata::instruction::{create_master_edition_v3, create_metadata_accounts_v2};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, program::invoke};

#[allow(clippy::too_many_arguments)]
pub fn metadata<'info>(
    authority: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    metadata: &AccountInfo<'info>,
    token_builder: TokenBuilder,
    system_program: &AccountInfo<'info>,
    rent_program: &AccountInfo<'info>,
    metadata_program: &AccountInfo<'info>,
) -> ProgramResult {
    let metadata_pubkey = pda::metadata(mint.key);
    invoke(
        &create_metadata_accounts_v2(
            mpl_token_metadata::ID,
            metadata_pubkey,
            *mint.key,
            *authority.key,
            *payer.key,
            *authority.key,
            token_builder.name,
            token_builder.symbol,
            token_builder.url,
            token_builder.creators,
            token_builder.seller_fee_basis_points,
            true,
            true,
            None,
            None,
        ),
        &[
            authority.clone(),
            mint.clone(),
            payer.clone(),
            metadata.clone(),
            system_program.clone(),
            rent_program.clone(),
            metadata_program.clone(),
        ],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn master_edition<'info>(
    authority: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    metadata: &AccountInfo<'info>,
    master_edition: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    rent_program: &AccountInfo<'info>,
    metadata_program: &AccountInfo<'info>,
) -> ProgramResult {
    let metadata_pubkey = pda::metadata(mint.key);
    let edition_pubkey = pda::master_edition(mint.key);

    invoke(
        &create_master_edition_v3(
            mpl_token_metadata::ID,
            edition_pubkey,
            *mint.key,
            *authority.key,
            *authority.key,
            metadata_pubkey,
            *payer.key,
            Some(1),
        ),
        &[
            authority.clone(),
            payer.clone(),
            mint.clone(),
            metadata.clone(),
            master_edition.clone(),
            system_program.clone(),
            rent_program.clone(),
            metadata_program.clone(),
        ],
    )
}
