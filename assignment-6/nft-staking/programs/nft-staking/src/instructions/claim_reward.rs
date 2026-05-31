use anchor_lang::{prelude::*, solana_program::clock::SECONDS_PER_DAY};

use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{mint_to_checked, MintToChecked},
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdatePluginV1CpiBuilder,
    types::{Attribute, Attributes, FreezeDelegate, Plugin, PluginType, UpdateAuthority},
};

use crate::{
    constants::UPDATE_AUTHORITY, error::StakingError, state::MplCore, StakeState, LAST_CLAIMED_AT,
    REWARDS_MINT, STAKED_KEY, STAKE_STATE,
};

#[derive(Accounts)]
pub struct ClaimReward<'info> {
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

    #[account(
        mut,
        seeds = [REWARDS_MINT, stake_state.key().as_ref()],
        bump = stake_state.reward_bump,
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner,
    )]
    pub user_reward_ata: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub mpl_core_program: Program<'info, MplCore>,
}

pub fn handler(ctx: Context<ClaimReward>) -> Result<()> {
    let freeze_plugin = fetch_plugin::<BaseAssetV1, FreezeDelegate>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::FreezeDelegate,
    )
    .ok()
    .map(|(_, plugin, _)| plugin);

    match freeze_plugin {
        Some(plugin) => {
            require!(plugin.frozen, StakingError::AssetNotStaked);
        }
        None => {
            return err!(StakingError::AssetNotStaked);
        }
    }

    let attributes = fetch_plugin::<BaseAssetV1, Attributes>(
        &ctx.accounts.asset.to_account_info(),
        PluginType::Attributes,
    )
    .ok()
    .map(|(_, attrs, _)| attrs)
    .ok_or(StakingError::MissingAttributes)?;

    let current_timestamp = Clock::get()?.unix_timestamp;

    let mut last_claimed_at: i64 = 0;

    let mut attributes_list: Vec<Attribute> = Vec::with_capacity(attributes.attribute_list.len());

    for attribute in &attributes.attribute_list {
        if attribute.key == STAKED_KEY {
            require!(attribute.value != "false", StakingError::AssetNotStaked);

            attributes_list.push(attribute.clone());
        } else if attribute.key == LAST_CLAIMED_AT {
            last_claimed_at = attribute
                .value
                .parse::<i64>()
                .map_err(|_| StakingError::InvalidTimestamp)?;
        } else {
            attributes_list.push(attribute.clone());
        }
    }

    require!(last_claimed_at > 0, StakingError::InvalidTimestamp);

    let elapsed_time = current_timestamp
        .checked_sub(last_claimed_at)
        .ok_or(StakingError::InvalidTimestamp)?;

    let staked_days = elapsed_time
        .checked_div(SECONDS_PER_DAY as i64)
        .ok_or(StakingError::InvalidTimestamp)?;

    attributes_list.retain(|attribute| attribute.key != LAST_CLAIMED_AT);

    attributes_list.push(Attribute {
        key: LAST_CLAIMED_AT.to_string(),
        value: current_timestamp.to_string(),
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

    UpdatePluginV1CpiBuilder::new(&ctx.accounts.mpl_core_program.to_account_info())
        .asset(&ctx.accounts.asset.to_account_info())
        .collection(Some(&ctx.accounts.collection.to_account_info()))
        .payer(&ctx.accounts.owner.to_account_info())
        .authority(Some(&ctx.accounts.update_authority.to_account_info()))
        .system_program(&ctx.accounts.system_program.to_account_info())
        .plugin(attributes_plugin)
        .invoke_signed(signer_seeds)?;

    let amount = (staked_days as u64)
        .checked_mul(ctx.accounts.stake_state.rewards_bps as u64)
        .ok_or(StakingError::InvalidRewardsBPS)?
        .checked_mul(10u64.pow(ctx.accounts.rewards_mint.decimals as u32))
        .ok_or(StakingError::InvalidRewardsBPS)?
        .checked_div(10_000)
        .ok_or(StakingError::InvalidRewardsBPS)?;

    let stake_signer_seeds: &[&[&[u8]]] = &[&[
        STAKE_STATE,
        collection_key.as_ref(),
        &[ctx.accounts.stake_state.bump],
    ]];

    let cpi_accounts = MintToChecked {
        mint: ctx.accounts.rewards_mint.to_account_info(),
        authority: ctx.accounts.stake_state.to_account_info(),
        to: ctx.accounts.user_reward_ata.to_account_info(),
    };

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        stake_signer_seeds,
    );

    mint_to_checked(cpi_ctx, amount, ctx.accounts.rewards_mint.decimals)?;

    Ok(())
}
