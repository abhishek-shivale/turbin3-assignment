use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{
        AddPluginV1CpiBuilder, UpdateCollectionPluginV1CpiBuilder, UpdatePluginV1CpiBuilder,
    },
    types::{
        Attribute, Attributes, FreezeDelegate, Plugin, PluginAuthority, PluginType, UpdateAuthority,
    },
};

use crate::{
    constants::UPDATE_AUTHORITY, error::StakingError, state::MplCore, StakeState, LAST_CLAIMED_AT,
    STAKED_AT, STAKED_COUNT, STAKED_KEY, STAKE_STATE,
};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [STAKE_STATE, collection.key().as_ref()],
        bump = stake_state.bump,
    )]
    pub stake_state: Box<Account<'info, StakeState>>,

    #[account(
        mut,
        has_one = owner @ StakingError::InvalidOwner,
        constraint = asset.update_authority == UpdateAuthority::Collection(collection.key()) @ StakingError::InvalidUpdateAuthority,
    )]
    pub asset: Box<Account<'info, BaseAssetV1>>,

    #[account(
        mut,
        has_one = update_authority @ StakingError::InvalidUpdateAuthority,
    )]
    pub collection: Box<Account<'info, BaseCollectionV1>>,

    #[account(
        seeds = [UPDATE_AUTHORITY, collection.key().as_ref()],
        bump
    )]
    pub update_authority: SystemAccount<'info>,

    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, MplCore>,
}

pub fn handler(ctx: Context<Stake>) -> Result<()> {
    let freeze_plugin = fetch_plugin::<BaseAssetV1, FreezeDelegate>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::FreezeDelegate,
    )
    .ok()
    .map(|(_, plugin, _)| plugin);

    if let Some(plugin) = freeze_plugin {
        require!(!plugin.frozen, StakingError::FrozenAsset);
    }

    let collection_attributes_plugin = fetch_plugin::<BaseCollectionV1, Attributes>(
        &ctx.accounts.collection.to_account_info(),
        PluginType::Attributes,
    )
    .ok()
    .map(|(_, plugin, _)| plugin);

    let mut staked_count: u64 = 0;
    let mut found = false;

    let mut collection_attributes: Vec<Attribute> = Vec::new();

    if let Some(plugin) = &collection_attributes_plugin {
        for attribute in &plugin.attribute_list {
            if attribute.key == STAKED_COUNT {
                found = true;

                staked_count = attribute
                    .value
                    .parse::<u64>()
                    .map_err(|_| StakingError::InvalidCollection)?;
            } else {
                collection_attributes.push(attribute.clone());
            }
        }
    }

    require!(found, StakingError::MissingStakedCount);

    staked_count = staked_count
        .checked_add(1)
        .ok_or(StakingError::MissingStakedCount)?;

    collection_attributes.push(Attribute {
        key: STAKED_COUNT.to_string(),
        value: staked_count.to_string(),
    });

    let collection_plugin = Plugin::Attributes(Attributes {
        attribute_list: collection_attributes,
    });

    let attributes_fetced = fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    )
    .ok()
    .map(|(_, attrs, _)| attrs);

    let mut attributes_list: Vec<Attribute> = Vec::new();

    if let Some(attributes) = &attributes_fetced {
        for attribute in &attributes.attribute_list {
            if attribute.key == STAKED_KEY {
                require!(attribute.value != "true", StakingError::AlreadyStaked);
            } else if attribute.key != STAKED_AT {
                attributes_list.push(attribute.clone());
            }
        }
    }

    let now = Clock::get()?.unix_timestamp;

    attributes_list.push(Attribute {
        key: STAKED_KEY.to_string(),
        value: "true".to_string(),
    });

    attributes_list.push(Attribute {
        key: STAKED_AT.to_string(),
        value: now.to_string(),
    });

    attributes_list.push(Attribute {
        key: LAST_CLAIMED_AT.to_string(),
        value: now.to_string(),
    });

    let attributes_plugin = Plugin::Attributes(Attributes {
        attribute_list: attributes_list,
    });

    let collection_key = ctx.accounts.collection.key();

    let signer_seeds: &[&[&[u8]]] = &[&[
        UPDATE_AUTHORITY,
        collection_key.as_ref(),
        &[ctx.bumps.update_authority],
    ]];

    if attributes_fetced.is_none() {
        AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.owner.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(attributes_plugin)
            .init_authority(PluginAuthority::UpdateAuthority)
            .invoke_signed(signer_seeds)?;
    } else {
        UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
            .asset(&ctx.accounts.asset.to_account_info())
            .collection(Some(&ctx.accounts.collection.to_account_info()))
            .payer(&ctx.accounts.owner.to_account_info())
            .authority(Some(&ctx.accounts.update_authority.to_account_info()))
            .system_program(&ctx.accounts.system_program.to_account_info())
            .plugin(attributes_plugin)
            .invoke_signed(signer_seeds)?;
    }

    UpdateCollectionPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .collection(&ctx.accounts.collection.to_account_info())
        .payer(&ctx.accounts.owner.to_account_info())
        .authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin(collection_plugin)
        .invoke_signed(signer_seeds)?;

    AddPluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.owner.to_account_info())
        .authority(Some(&ctx.accounts.owner.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
        .init_authority(PluginAuthority::Owner)
        .invoke()?;

    Ok(())
}
