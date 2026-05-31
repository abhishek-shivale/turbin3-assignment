use anchor_lang::prelude::*;
use mpl_core::{
    instructions::{CreateCollectionV2CpiBuilder},
    types::{
        Attribute, Attributes, Plugin, PluginAuthority, PluginAuthorityPair,
    },
};

use crate::{constants::UPDATE_AUTHORITY, state::MplCore, STAKED_COUNT};

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub collection: Signer<'info>,

    #[account(
        seeds = [UPDATE_AUTHORITY, collection.key().as_ref()],
        bump
    )]
    pub update_authority: SystemAccount<'info>,

    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, MplCore>,
}

pub fn handler(ctx: Context<CreateCollection>, name: String, uri: String) -> Result<()> {
    let collection_key = ctx.accounts.collection.key();

    let signer_seeds: &[&[&[u8]]] = &[&[
        UPDATE_AUTHORITY,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ]];

    let plugins = vec![PluginAuthorityPair {
        plugin: Plugin::Attributes(Attributes {
            attribute_list: vec![Attribute {
                key: STAKED_COUNT.to_string(),
                value: "0".to_string(),
            }],
        }),
        authority: Some(PluginAuthority::UpdateAuthority),
    }];


    CreateCollectionV2CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .collection(&ctx.accounts.collection.to_account_info())
        .payer(&ctx.accounts.payer.to_account_info())
        .update_authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugins(plugins)
        .name(name)
        .uri(uri)
        .invoke_signed(signer_seeds)?;
    
    Ok(())
}
