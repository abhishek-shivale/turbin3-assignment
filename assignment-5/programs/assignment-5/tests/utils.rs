use {
    litesvm::LiteSVM,
    litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo},
    solana_keypair::{Keypair, Signer},
    solana_pubkey::Pubkey,
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

pub fn create_user_ata(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    _owner: &Pubkey,
) -> Pubkey {
    CreateAssociatedTokenAccount::new(svm, payer, mint)
        .send()
        .unwrap()
}

pub fn mint_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    to: &Pubkey,
    amount: u64,
    _authority: &Keypair,
) {
    MintTo::new(svm, payer, mint, to, amount)
        .send()
        .unwrap();
}

pub fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    use anchor_spl::associated_token::get_associated_token_address;
    get_associated_token_address(owner, mint)
}
