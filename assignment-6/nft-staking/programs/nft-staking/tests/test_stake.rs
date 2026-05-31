mod helpers;

use helpers::*;

#[test]
fn stake_freezes_asset_and_updates_attributes() {
    let mut svm = init_svm();
    let setup = setup_collection(&mut svm, DEFAULT_REWARDS_BPS, DEFAULT_FREEZE_PERIOD_DAYS);
    let user = funded_keypair(&mut svm);
    let asset = mint_asset(&mut svm, &setup.collection_key(), &user);

    let ix = stake_ix(&user.pubkey(), &asset.pubkey(), &setup.collection_key());
    assert_ok(send_ix(&mut svm, &[&user], ix));

    // Asset is frozen via the FreezeDelegate plugin.
    assert!(asset_frozen(&svm, &asset.pubkey()));

    // Asset attributes reflect the staked state.
    let attrs = asset_attributes(&svm, &asset.pubkey());
    assert_eq!(attr_value(&attrs, STAKED_KEY), Some("true".to_string()));
    assert!(attr_value(&attrs, STAKED_AT).is_some());
    assert!(attr_value(&attrs, LAST_CLAIMED_AT).is_some());

    // Collection staked_count incremented.
    let collection_attrs = collection_attributes(&svm, &setup.collection_key());
    assert_eq!(attr_value(&collection_attrs, STAKED_COUNT), Some("1".to_string()));
}
