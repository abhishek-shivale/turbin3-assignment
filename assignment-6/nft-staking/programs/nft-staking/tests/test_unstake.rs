mod helpers;

use helpers::*;

// rewards_bps = 10_000 (100%), MINT_DECIMALS = 6:
// amount = days * bps * 10^6 / 10_000 = days * 1_000_000
const REWARD_PER_DAY: u64 = 1_000_000;

#[test]
fn unstake_unfreezes_asset_and_pays_reward() {
    let mut svm = init_svm();
    let setup = setup_collection(&mut svm, DEFAULT_REWARDS_BPS, DEFAULT_FREEZE_PERIOD_DAYS);
    let user = funded_keypair(&mut svm);
    let asset = stake_asset(&mut svm, &setup.collection_key(), &user);

    warp_days(&mut svm, 3);

    let ix = unstake_ix(&user.pubkey(), &asset.pubkey(), &setup.collection_key());
    assert_ok(send_ix(&mut svm, &[&user], ix));

    // FreezeDelegate plugin removed -> not frozen.
    assert!(!asset_frozen(&svm, &asset.pubkey()));

    // Asset marked unstaked.
    let attrs = asset_attributes(&svm, &asset.pubkey());
    assert_eq!(attr_value(&attrs, STAKED_KEY), Some("false".to_string()));

    // Collection staked_count back to zero.
    let collection_attrs = collection_attributes(&svm, &setup.collection_key());
    assert_eq!(attr_value(&collection_attrs, STAKED_COUNT), Some("0".to_string()));

    // Reward paid for the elapsed days.
    let ata = rewards_ata(&user.pubkey(), &setup.rewards_mint);
    assert_eq!(token_balance(&svm, &ata), 3 * REWARD_PER_DAY);
}
