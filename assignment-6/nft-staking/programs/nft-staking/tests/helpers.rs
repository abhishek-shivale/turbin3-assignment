#![allow(dead_code, unused_imports)]

pub use solana_sdk::signature::{Keypair, Signer};

use {
    anchor_lang::{
        prelude::*, system_program::ID as SYSTEM_PROGRAM_ID, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{get_associated_token_address, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::ID as TOKEN_PROGRAM_ID,
    },
    litesvm::{types::TransactionResult, LiteSVM},
    mpl_core::{
        accounts::{BaseAssetV1, BaseCollectionV1, PluginHeaderV1},
        types::{Attribute, Attributes, FreezeDelegate, Plugin},
        DataBlob, PluginRegistryV1Safe, ID as MPL_CORE_PROGRAM_ID,
    },
    nft_staking::{
        accounts as ix_accounts, error::StakingError, instruction as ix_data, state::StakeState,
        REWARDS_MINT, STAKE_STATE, UPDATE_AUTHORITY,
    },
    solana_sdk::{
        clock::{Clock, SECONDS_PER_DAY},
        instruction::{Instruction, InstructionError},
        message::Message,
        transaction::{Transaction, TransactionError},
    },
};

pub use mpl_core::types::UpdateAuthority;
pub use nft_staking::{LAST_CLAIMED_AT, STAKED_AT, STAKED_COUNT, STAKED_KEY};

pub const INITIAL_USER_LAMPORTS: u64 = 5_000_000_000;
pub const DEFAULT_REWARDS_BPS: u16 = 10_000;
pub const DEFAULT_FREEZE_PERIOD_DAYS: u16 = 0;

pub const COLLECTION_NAME: &str = "Test Collection";
pub const COLLECTION_URI: &str = "https://example.com/collection.json";
pub const ASSET_NAME: &str = "Test Asset";
pub const ASSET_URI: &str = "https://example.com/asset.json";

pub fn init_svm() -> LiteSVM {
    let mut svm = LiteSVM::new();

    svm.add_program_from_file(nft_staking::id(), "../../target/deploy/nft_staking.so")
        .unwrap();

    svm.add_program_from_file(MPL_CORE_PROGRAM_ID, "../../target/deploy/mpl_core.so")
        .unwrap_or_else(|e| {
            panic!(
                "load mpl-core failed: {e:?}\n\run `anchor test` once to trigger the pre-test fetch hook"
            )
        });

    let mut clock: Clock = svm.get_sysvar();
    clock.unix_timestamp = 1_000_000_000;
    svm.set_sysvar(&clock);

    svm
}

pub fn airdrop(svm: &mut LiteSVM, recipient: &Pubkey, lamports: u64) {
    svm.airdrop(recipient, lamports).unwrap();
}

pub fn funded_keypair(svm: &mut LiteSVM) -> Keypair {
    let kp = Keypair::new();
    airdrop(svm, &kp.pubkey(), INITIAL_USER_LAMPORTS);
    kp
}

pub fn stake_state_pda(collection: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STAKE_STATE, collection.as_ref()], &nft_staking::ID)
}

pub fn update_authority_pda(collection: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[UPDATE_AUTHORITY, collection.as_ref()], &nft_staking::ID)
}

pub fn rewards_mint_pda(stake_state: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REWARDS_MINT, stake_state.as_ref()], &nft_staking::ID)
}

pub fn rewards_ata(owner: &Pubkey, rewards_mint: &Pubkey) -> Pubkey {
    get_associated_token_address(owner, rewards_mint)
}

pub fn create_collection_ix(
    payer: &Pubkey,
    collection: &Pubkey,
    name: impl Into<String>,
    uri: impl Into<String>,
) -> Instruction {
    let (update_authority, _) = update_authority_pda(collection);
    Instruction {
        program_id: nft_staking::ID,
        accounts: ix_accounts::CreateCollection {
            payer: *payer,
            collection: *collection,
            update_authority,
            system_program: SYSTEM_PROGRAM_ID,
            mpl_core_program: MPL_CORE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: ix_data::CreateCollection {
            name: name.into(),
            uri: uri.into(),
        }
        .data(),
    }
}

pub fn init_ix(
    admin: &Pubkey,
    collection: &Pubkey,
    rewards_bps: u16,
    freeze_period: u16,
) -> Instruction {
    let (stake_state, _) = stake_state_pda(collection);
    let (update_authority, _) = update_authority_pda(collection);
    let (rewards_mint, _) = rewards_mint_pda(&stake_state);
    Instruction {
        program_id: nft_staking::ID,
        accounts: ix_accounts::Initialize {
            admin: *admin,
            stake_state,
            collection: *collection,
            update_authority,
            rewards_mint,
            system_program: SYSTEM_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: ix_data::Initialize {
            rewards_bps,
            freeze_period,
        }
        .data(),
    }
}

pub fn mint_asset_ix(
    user: &Pubkey,
    asset: &Pubkey,
    collection: &Pubkey,
    name: impl Into<String>,
    uri: impl Into<String>,
) -> Instruction {
    let (update_authority, _) = update_authority_pda(collection);
    Instruction {
        program_id: nft_staking::ID,
        accounts: ix_accounts::MintAsset {
            user: *user,
            asset: *asset,
            collection: *collection,
            update_authority,
            system_program: SYSTEM_PROGRAM_ID,
            mpl_core_program: MPL_CORE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: ix_data::MintAsset {
            name: name.into(),
            uri: uri.into(),
        }
        .data(),
    }
}

pub fn stake_ix(owner: &Pubkey, asset: &Pubkey, collection: &Pubkey) -> Instruction {
    let (stake_state, _) = stake_state_pda(collection);
    let (update_authority, _) = update_authority_pda(collection);
    Instruction {
        program_id: nft_staking::ID,
        accounts: ix_accounts::Stake {
            owner: *owner,
            stake_state,
            asset: *asset,
            collection: *collection,
            update_authority,
            system_program: SYSTEM_PROGRAM_ID,
            mpl_core_program: MPL_CORE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: ix_data::Stake {}.data(),
    }
}

pub fn unstake_ix(owner: &Pubkey, asset: &Pubkey, collection: &Pubkey) -> Instruction {
    let (stake_state, _) = stake_state_pda(collection);
    let (update_authority, _) = update_authority_pda(collection);
    let (rewards_mint, _) = rewards_mint_pda(&stake_state);
    let user_reward_ata = rewards_ata(owner, &rewards_mint);
    Instruction {
        program_id: nft_staking::ID,
        accounts: ix_accounts::Unstake {
            owner: *owner,
            stake_state,
            asset: *asset,
            collection: *collection,
            update_authority,
            rewards_mint,
            user_reward_ata,
            system_program: SYSTEM_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            mpl_core_program: MPL_CORE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: ix_data::Unstake {}.data(),
    }
}

pub fn claim_reward_ix(owner: &Pubkey, asset: &Pubkey, collection: &Pubkey) -> Instruction {
    let (stake_state, _) = stake_state_pda(collection);
    let (update_authority, _) = update_authority_pda(collection);
    let (rewards_mint, _) = rewards_mint_pda(&stake_state);
    let user_reward_ata = rewards_ata(owner, &rewards_mint);
    Instruction {
        program_id: nft_staking::ID,
        accounts: ix_accounts::ClaimReward {
            owner: *owner,
            stake_state,
            asset: *asset,
            collection: *collection,
            update_authority,
            rewards_mint,
            user_reward_ata,
            system_program: SYSTEM_PROGRAM_ID,
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            mpl_core_program: MPL_CORE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: ix_data::ClaimReward {}.data(),
    }
}

pub fn send_ix(svm: &mut LiteSVM, signers: &[&Keypair], ix: Instruction) -> TransactionResult {
    send_ixs(svm, signers, &[ix])
}

pub fn send_ixs(svm: &mut LiteSVM, signers: &[&Keypair], ixs: &[Instruction]) -> TransactionResult {
    let payer = signers.first().expect("at least one signer required");
    let message = Message::new(ixs, Some(&payer.pubkey()));
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new(signers, message, blockhash);
    let result = svm.send_transaction(tx);
    svm.expire_blockhash();
    result
}

pub fn fetch_stake_state(svm: &LiteSVM, stake_state: &Pubkey) -> StakeState {
    let account = svm
        .get_account(stake_state)
        .expect("stake_state account missing");
    StakeState::try_deserialize(&mut account.data.as_ref()).expect("decode StakeState")
}

pub fn fetch_collection(svm: &LiteSVM, collection: &Pubkey) -> BaseCollectionV1 {
    let account = svm
        .get_account(collection)
        .expect("collection account missing");
    BaseCollectionV1::from_bytes(&account.data).expect("decode BaseCollectionV1")
}

pub fn fetch_asset(svm: &LiteSVM, asset: &Pubkey) -> BaseAssetV1 {
    let account = svm.get_account(asset).expect("asset account missing");
    BaseAssetV1::from_bytes(&account.data).expect("decode BaseAssetV1")
}

pub fn token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    svm.get_account(ata)
        .filter(|acc| acc.data.len() >= 72)
        .map(|acc| {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&acc.data[64..72]);
            u64::from_le_bytes(buf)
        })
        .unwrap_or(0)
}

pub fn assert_ok(result: TransactionResult) {
    if let Err(failed) = result {
        panic!("expected success, got error:\n{:#?}", failed.meta.logs);
    }
}

pub fn assert_staking_error(result: TransactionResult, expected: StakingError) {
    let expected_code: u32 = expected.into();
    match result {
        Ok(meta) => panic!(
            "expected `{expected:?}` (code {expected_code}), but transaction succeeded.\n\
             logs:\n{:#?}",
            meta.logs
        ),
        Err(failed) => match failed.err {
            TransactionError::InstructionError(_, InstructionError::Custom(code)) => assert_eq!(
                code, expected_code,
                "expected `{expected:?}` (code {expected_code}), got code {code}.\n\
                 logs:\n{:#?}",
                failed.meta.logs
            ),
            other => panic!(
                "expected `{expected:?}`, got structural error: {other:?}.\n\
                 logs:\n{:#?}",
                failed.meta.logs
            ),
        },
    }
}

pub fn warp_days(svm: &mut LiteSVM, days: i64) {
    let mut clock: Clock = svm.get_sysvar();
    let seconds = days
        .checked_mul(SECONDS_PER_DAY as i64)
        .expect("days overflow");
    clock.unix_timestamp = clock
        .unix_timestamp
        .checked_add(seconds)
        .expect("timestamp overflow");
    svm.set_sysvar(&clock);
}

pub struct StakingCollection {
    pub admin: Keypair,
    pub collection: Keypair,
    pub stake_state: Pubkey,
    pub update_authority: Pubkey,
    pub rewards_mint: Pubkey,
}

impl StakingCollection {
    pub fn collection_key(&self) -> Pubkey {
        self.collection.pubkey()
    }
}

pub fn setup_collection(
    svm: &mut LiteSVM,
    rewards_bps: u16,
    freeze_period: u16,
) -> StakingCollection {
    let admin = funded_keypair(svm);
    let collection = Keypair::new();
    let collection_key = collection.pubkey();

    let ix = create_collection_ix(
        &admin.pubkey(),
        &collection_key,
        COLLECTION_NAME,
        COLLECTION_URI,
    );
    assert_ok(send_ix(svm, &[&admin, &collection], ix));

    let ix = init_ix(&admin.pubkey(), &collection_key, rewards_bps, freeze_period);
    assert_ok(send_ix(svm, &[&admin], ix));

    let (stake_state, _) = stake_state_pda(&collection_key);
    let (update_authority, _) = update_authority_pda(&collection_key);
    let (rewards_mint, _) = rewards_mint_pda(&stake_state);

    StakingCollection {
        admin,
        collection,
        stake_state,
        update_authority,
        rewards_mint,
    }
}

pub fn mint_asset(svm: &mut LiteSVM, collection: &Pubkey, owner: &Keypair) -> Keypair {
    let asset = Keypair::new();
    let ix = mint_asset_ix(
        &owner.pubkey(),
        &asset.pubkey(),
        collection,
        ASSET_NAME,
        ASSET_URI,
    );
    assert_ok(send_ix(svm, &[owner, &asset], ix));
    asset
}

/// Stake an asset and return its keypair.
pub fn stake_asset(svm: &mut LiteSVM, collection: &Pubkey, owner: &Keypair) -> Keypair {
    let asset = mint_asset(svm, collection, owner);
    let ix = stake_ix(&owner.pubkey(), &asset.pubkey(), collection);
    assert_ok(send_ix(svm, &[owner], ix));
    asset
}

/// Decode the `Attributes` plugin attribute list from a raw mpl-core account.
fn decode_attribute_list(data: &[u8], base_len: usize) -> Vec<Attribute> {
    if base_len >= data.len() {
        return Vec::new();
    }
    let header = PluginHeaderV1::from_bytes(&data[base_len..]).expect("decode PluginHeaderV1");
    let registry =
        PluginRegistryV1Safe::from_bytes(&data[header.plugin_registry_offset as usize..])
            .expect("decode PluginRegistryV1Safe");
    for record in &registry.registry {
        if let Ok(Plugin::Attributes(Attributes { attribute_list })) =
            Plugin::deserialize(&mut &data[record.offset as usize..])
        {
            return attribute_list;
        }
    }
    Vec::new()
}

/// Read the asset's `Attributes` plugin (empty if missing).
pub fn asset_attributes(svm: &LiteSVM, asset: &Pubkey) -> Vec<Attribute> {
    let account = svm.get_account(asset).expect("asset account missing");
    let base = BaseAssetV1::from_bytes(&account.data).expect("decode BaseAssetV1");
    decode_attribute_list(&account.data, base.len())
}

/// Read the collection's `Attributes` plugin (empty if missing).
pub fn collection_attributes(svm: &LiteSVM, collection: &Pubkey) -> Vec<Attribute> {
    let account = svm.get_account(collection).expect("collection account missing");
    let base = BaseCollectionV1::from_bytes(&account.data).expect("decode BaseCollectionV1");
    decode_attribute_list(&account.data, base.len())
}

/// Look up a single attribute value by key.
pub fn attr_value(attrs: &[Attribute], key: &str) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.key == key)
        .map(|a| a.value.clone())
}

/// True if the asset has a `FreezeDelegate` plugin in the frozen state.
pub fn asset_frozen(svm: &LiteSVM, asset: &Pubkey) -> bool {
    let account = svm.get_account(asset).expect("asset account missing");
    let base = BaseAssetV1::from_bytes(&account.data).expect("decode BaseAssetV1");
    if base.len() >= account.data.len() {
        return false;
    }
    let header =
        PluginHeaderV1::from_bytes(&account.data[base.len()..]).expect("decode PluginHeaderV1");
    let registry =
        PluginRegistryV1Safe::from_bytes(&account.data[header.plugin_registry_offset as usize..])
            .expect("decode PluginRegistryV1Safe");
    for record in &registry.registry {
        if let Ok(Plugin::FreezeDelegate(FreezeDelegate { frozen })) =
            Plugin::deserialize(&mut &account.data[record.offset as usize..])
        {
            return frozen;
        }
    }
    false
}
