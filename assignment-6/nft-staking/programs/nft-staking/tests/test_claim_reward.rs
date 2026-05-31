mod helpers;

use helpers::*;

// rewards_bps = 10_000 (100%), MINT_DECIMALS = 6:
// amount = days * bps * 10^6 / 10_000 = days * 1_000_000
const REWARD_PER_DAY: u64 = 1_000_000;

#[test]
fn claim_reward_mints_tokens_and_keeps_asset_staked() {
    let mut svm = init_svm();
    let setup = setup_collection(&mut svm, DEFAULT_REWARDS_BPS, DEFAULT_FREEZE_PERIOD_DAYS);
    let user = funded_keypair(&mut svm);
    let asset = stake_asset(&mut svm, &setup.collection_key(), &user);

    warp_days(&mut svm, 5);

    let ix = claim_reward_ix(&user.pubkey(), &asset.pubkey(), &setup.collection_key());
    assert_ok(send_ix(&mut svm, &[&user], ix));

    let ata = rewards_ata(&user.pubkey(), &setup.rewards_mint);
    assert_eq!(token_balance(&svm, &ata), 5 * REWARD_PER_DAY);

    // Asset stays staked and frozen after claiming.
    assert!(asset_frozen(&svm, &asset.pubkey()));
    let attrs = asset_attributes(&svm, &asset.pubkey());
    assert_eq!(attr_value(&attrs, STAKED_KEY), Some("true".to_string()));
}
