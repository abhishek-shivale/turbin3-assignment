use {
    litesvm::LiteSVM, litesvm_token::CreateMint, solana_keypair::{Keypair, Signer}, solana_pubkey::Pubkey
};

pub fn setup() -> (LiteSVM, Keypair) {
    let program_id = assignment_5::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/assignment_5.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    (svm, payer)
}

pub fn create_mint(svm: &mut LiteSVM, authority: &Keypair) -> Pubkey {
    CreateMint::new(svm, authority)
        .decimals(2)
        .authority(&authority.pubkey())
        .send()
        .unwrap()
}

