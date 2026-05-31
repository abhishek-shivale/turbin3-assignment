use anchor_lang::prelude::*;
use mpl_core::{accounts::BaseCollectionV1, instructions::CreateV2CpiBuilder};

use crate::{constants::UPDATE_AUTHORITY, state::MplCore};

#[derive(Accounts)]
pub struct MintAsset<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub asset: Signer<'info>,

    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        seeds = [UPDATE_AUTHORITY, collection.key().as_ref()],
        bump
    )]
    pub update_authority: SystemAccount<'info>,

    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, MplCore>,
}

pub fn handler(ctx: Context<MintAsset>, name: String, uri: String) -> Result<()> {
    let collection_key = ctx.accounts.collection.key();

    let signer_seeds: &[&[&[u8]]] = &[&[
        UPDATE_AUTHORITY,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ]];

    CreateV2CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .update_authority(None)
        .payer(&ctx.accounts.user.to_account_info())
        .owner(Some(&ctx.accounts.user.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .name(name)
        .uri(uri)
        .invoke_signed(signer_seeds)?;

    Ok(())
}

