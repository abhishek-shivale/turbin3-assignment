mod helpers;

use helpers::*;

#[test]
fn create_collection_sets_metadata_and_staked_count() {
    let mut svm = init_svm();
    let admin = funded_keypair(&mut svm);
    let collection = Keypair::new();
    let collection_key = collection.pubkey();
    let (update_authority, _) = update_authority_pda(&collection_key);

    let ix = create_collection_ix(
        &admin.pubkey(),
        &collection_key,
        COLLECTION_NAME,
        COLLECTION_URI,
    );
    assert_ok(send_ix(&mut svm, &[&admin, &collection], ix));

    let collection_account = fetch_collection(&svm, &collection_key);
    assert_eq!(collection_account.name, COLLECTION_NAME);
    assert_eq!(collection_account.uri, COLLECTION_URI);
    assert_eq!(collection_account.update_authority, update_authority);
    assert_eq!(collection_account.num_minted, 0);
    assert_eq!(collection_account.current_size, 0);

    let attrs = collection_attributes(&svm, &collection_key);
    assert_eq!(attr_value(&attrs, STAKED_COUNT), Some("0".to_string()));
}
