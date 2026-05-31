mod helpers;

use helpers::*;

#[test]
fn mint_asset_creates_asset_owned_by_user() {
    let mut svm = init_svm();
    let setup = setup_collection(&mut svm, DEFAULT_REWARDS_BPS, DEFAULT_FREEZE_PERIOD_DAYS);
    let user = funded_keypair(&mut svm);

    let asset = mint_asset(&mut svm, &setup.collection_key(), &user);

    let asset_account = fetch_asset(&svm, &asset.pubkey());
    assert_eq!(asset_account.owner, user.pubkey());
    assert_eq!(
        asset_account.update_authority,
        UpdateAuthority::Collection(setup.collection_key())
    );
    assert_eq!(asset_account.name, ASSET_NAME);
    assert_eq!(asset_account.uri, ASSET_URI);

    let collection_account = fetch_collection(&svm, &setup.collection_key());
    assert_eq!(collection_account.num_minted, 1);
    assert_eq!(collection_account.current_size, 1);
}
