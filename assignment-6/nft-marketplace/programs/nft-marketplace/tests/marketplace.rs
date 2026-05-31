use anchor_lang::solana_program::{instruction::Instruction, pubkey::Pubkey};
use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_spl::associated_token::{self, spl_associated_token_account};
use anchor_spl::token::spl_token;
use litesvm::LiteSVM;
use solana_sdk::{
    signature::Keypair, signer::Signer, system_instruction, system_program,
    transaction::Transaction,
};

const MINT_LEN: u64 = 82;
const MARKETPLACE_SEED: &[u8] = b"marketplace";
const TREASURY_SEED: &[u8] = b"treasury";
const REWARDS_SEED: &[u8] = b"rewards";

fn program_id() -> Pubkey {
    nft_marketplace::ID
}

fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let payer = Keypair::new();
    svm.add_program_from_file(program_id(), "../../target/deploy/nft_marketplace.so")
        .unwrap();
    svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
    (svm, payer)
}

fn send(svm: &mut LiteSVM, payer: &Keypair, ixs: &[Instruction], signers: &[&Keypair]) {
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), signers, blockhash);
    if let Err(e) = svm.send_transaction(tx) {
        panic!("tx failed: {:?}", e.meta.logs);
    }
}

fn marketplace_pda(name: &str) -> Pubkey {
    Pubkey::find_program_address(&[MARKETPLACE_SEED, name.as_bytes()], &program_id()).0
}

fn treasury_pda(marketplace: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[TREASURY_SEED, marketplace.as_ref()], &program_id()).0
}

fn rewards_pda(marketplace: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[REWARDS_SEED, marketplace.as_ref()], &program_id()).0
}

fn listing_pda(marketplace: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[marketplace.as_ref(), mint.as_ref()], &program_id()).0
}

fn ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    associated_token::get_associated_token_address(owner, mint)
}

fn create_mint(svm: &mut LiteSVM, payer: &Keypair, authority: &Pubkey, decimals: u8) -> Pubkey {
    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(MINT_LEN as usize);
    let create = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        MINT_LEN,
        &spl_token::ID,
    );
    let init = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint.pubkey(),
        authority,
        None,
        decimals,
    )
    .unwrap();
    send(svm, payer, &[create, init], &[payer, &mint]);
    mint.pubkey()
}

fn create_ata(svm: &mut LiteSVM, payer: &Keypair, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        owner,
        mint,
        &spl_token::ID,
    );
    send(svm, payer, &[ix], &[payer]);
    ata(owner, mint)
}

fn mint_to(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    dest: &Pubkey,
    authority: &Keypair,
    amount: u64,
) {
    let ix =
        spl_token::instruction::mint_to(&spl_token::ID, mint, dest, &authority.pubkey(), &[], amount)
            .unwrap();
    send(svm, payer, &[ix], &[payer, authority]);
}

fn token_balance(svm: &LiteSVM, token_account: &Pubkey) -> u64 {
    let acc = svm.get_account(token_account).unwrap();
    u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
}

fn init_marketplace(svm: &mut LiteSVM, payer: &Keypair, fee: u16) -> (String, Pubkey, Pubkey) {
    let name = "turbin3".to_string();
    let marketplace = marketplace_pda(&name);
    let treasury = treasury_pda(&marketplace);
    let rewards_mint = rewards_pda(&marketplace);
    let payment_mint = create_mint(svm, payer, &payer.pubkey(), 6);

    let data = nft_marketplace::instruction::Init {
        name: name.clone(),
        fee,
    }
    .data();
    let metas = nft_marketplace::accounts::Init {
        admin: payer.pubkey(),
        marketplace,
        treasury,
        payment_mint,
        rewards_mint,
        token_program: spl_token::ID,
        system_program: system_program::ID,
    }
    .to_account_metas(None);

    send(
        svm,
        payer,
        &[Instruction::new_with_bytes(program_id(), &data, metas)],
        &[payer],
    );
    (name, marketplace, payment_mint)
}

fn mint_nft(svm: &mut LiteSVM, payer: &Keypair, owner: &Keypair) -> Pubkey {
    let nft = create_mint(svm, payer, &payer.pubkey(), 0);
    let owner_ata = create_ata(svm, payer, &owner.pubkey(), &nft);
    mint_to(svm, payer, &nft, &owner_ata, payer, 1);
    nft
}

fn list_nft(
    svm: &mut LiteSVM,
    payer: &Keypair,
    maker: &Keypair,
    marketplace: &Pubkey,
    nft: &Pubkey,
    price: u64,
) {
    let listing = listing_pda(marketplace, nft);
    let data = nft_marketplace::instruction::List { price }.data();
    let metas = nft_marketplace::accounts::List {
        maker: maker.pubkey(),
        marketplace: *marketplace,
        maker_mint: *nft,
        maker_ata: ata(&maker.pubkey(), nft),
        listing,
        vault: ata(&listing, nft),
        associated_token_program: spl_associated_token_account::ID,
        token_program: spl_token::ID,
        system_program: system_program::ID,
    }
    .to_account_metas(None);
    send(
        svm,
        payer,
        &[Instruction::new_with_bytes(program_id(), &data, metas)],
        &[payer, maker],
    );
}

#[test]
fn test_init() {
    let (mut svm, payer) = setup();
    let (_name, marketplace, _payment_mint) = init_marketplace(&mut svm, &payer, 250);
    let acc = svm.get_account(&marketplace).unwrap();
    assert_eq!(acc.owner, program_id());
}

#[test]
fn test_list() {
    let (mut svm, payer) = setup();
    let (_name, marketplace, _pm) = init_marketplace(&mut svm, &payer, 250);

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();
    let nft = mint_nft(&mut svm, &payer, &maker);

    list_nft(&mut svm, &payer, &maker, &marketplace, &nft, 1_000_000_000);

    let listing = listing_pda(&marketplace, &nft);
    let vault = ata(&listing, &nft);
    assert_eq!(token_balance(&svm, &vault), 1);
    assert_eq!(token_balance(&svm, &ata(&maker.pubkey(), &nft)), 0);
}

#[test]
fn test_delist() {
    let (mut svm, payer) = setup();
    let (_name, marketplace, _pm) = init_marketplace(&mut svm, &payer, 250);

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();
    let nft = mint_nft(&mut svm, &payer, &maker);
    list_nft(&mut svm, &payer, &maker, &marketplace, &nft, 1_000_000_000);

    let listing = listing_pda(&marketplace, &nft);
    let data = nft_marketplace::instruction::Delist {}.data();
    let metas = nft_marketplace::accounts::Delist {
        maker: maker.pubkey(),
        marketplace,
        maker_mint: nft,
        maker_ata: ata(&maker.pubkey(), &nft),
        listing,
        vault: ata(&listing, &nft),
        token_program: spl_token::ID,
    }
    .to_account_metas(None);
    send(
        &mut svm,
        &payer,
        &[Instruction::new_with_bytes(program_id(), &data, metas)],
        &[&payer, &maker],
    );

    assert_eq!(token_balance(&svm, &ata(&maker.pubkey(), &nft)), 1);
    assert!(svm
        .get_account(&listing)
        .map(|a| a.data.is_empty())
        .unwrap_or(true));
}

#[test]
fn test_buy_with_sol() {
    let (mut svm, payer) = setup();
    let (_name, marketplace, _pm) = init_marketplace(&mut svm, &payer, 250);
    let treasury = treasury_pda(&marketplace);
    let rewards_mint = rewards_pda(&marketplace);

    let maker = Keypair::new();
    let buyer = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&buyer.pubkey(), 10_000_000_000).unwrap();

    let nft = mint_nft(&mut svm, &payer, &maker);
    let price = 2_000_000_000u64;
    list_nft(&mut svm, &payer, &maker, &marketplace, &nft, price);

    let listing = listing_pda(&marketplace, &nft);
    let maker_before = svm.get_account(&maker.pubkey()).unwrap().lamports;

    let data = nft_marketplace::instruction::BuyWithSol {}.data();
    let metas = nft_marketplace::accounts::BuyWithSol {
        buyer: buyer.pubkey(),
        maker: maker.pubkey(),
        marketplace,
        treasury,
        maker_mint: nft,
        buyer_ata: ata(&buyer.pubkey(), &nft),
        listing,
        vault: ata(&listing, &nft),
        rewards_mint,
        buyer_rewards_ata: ata(&buyer.pubkey(), &rewards_mint),
        associated_token_program: spl_associated_token_account::ID,
        token_program: spl_token::ID,
        system_program: system_program::ID,
    }
    .to_account_metas(None);
    send(
        &mut svm,
        &buyer,
        &[Instruction::new_with_bytes(program_id(), &data, metas)],
        &[&buyer],
    );

    assert_eq!(token_balance(&svm, &ata(&buyer.pubkey(), &nft)), 1);
    assert_eq!(
        token_balance(&svm, &ata(&buyer.pubkey(), &rewards_mint)),
        1_000_000
    );
    let fee = price * 250 / 10_000;
    assert!(svm.get_account(&treasury).unwrap().lamports >= fee);
    let maker_after = svm.get_account(&maker.pubkey()).unwrap().lamports;
    assert!(maker_after >= maker_before + (price - fee));
}
