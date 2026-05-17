use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{get_associated_token_address, ID as ATA_PROGRAM_ID},
        token::{spl_token, TokenAccount},
    },
    escrow::state::Escrow,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    spl_associated_token_account::instruction::create_associated_token_account,
};

const MINT_LEN: u64 = 82;
const MINT_RENT: u64 = 1_461_600;

fn setup() -> (LiteSVM, Keypair) {
    let program_id = escrow::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/escrow.so");
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
}

fn send(svm: &mut LiteSVM, ixs: &[Instruction], payer: &Keypair, signers: &[&Keypair]) {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), signers).unwrap();
    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "tx failed: {:?}", res.err());
}

fn create_mint(svm: &mut LiteSVM, payer: &Keypair, mint: &Keypair) {
    // here we create the account use system (its raw account)
    let create = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        MINT_RENT,
        MINT_LEN,
        &spl_token::id(),
    );
    // here we turn raw account into spl-token 
    let init = spl_token::instruction::initialize_mint2(
        &spl_token::id(),
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        6,
    )
    .unwrap();
    send(svm, &[create, init], payer, &[payer, mint]);
}

fn create_ata_and_mint_to(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) -> Pubkey {
    // ATA address from (owner, mint). no oncahin call
    // we just derive the ata but it dosnt exits yet so we have to create it
    let ata = get_associated_token_address(owner, mint);
    // create account at that address. Payer funds rent. Owner becomes account authority.
    let create_ata =
        create_associated_token_account(&payer.pubkey(), owner, mint, &spl_token::id());
    let mint_to = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        &ata,
        &payer.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    send(svm, &[create_ata, mint_to], payer, &[payer]);
    ata
}

struct EscrowCtx {
    escrow_pda: Pubkey,
    vault: Pubkey,
    maker_ata_a: Pubkey,
    mint_a: Keypair,
    mint_b: Keypair,
    seed: u64,
    receive: u64,
    amount: u64,
    maker_initial: u64,
}

fn setup_with_make(svm: &mut LiteSVM, maker: &Keypair) -> EscrowCtx {
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    create_mint(svm, maker, &mint_a);
    create_mint(svm, maker, &mint_b);

    let maker_initial: u64 = 10_000_000;
    let maker_ata_a = create_ata_and_mint_to(
        svm,
        maker,
        &mint_a.pubkey(),
        &maker.pubkey(),
        maker_initial,
    );

    let seed: u64 = 1;
    let receive: u64 = 1_000_000;
    let amount: u64 = 500_000;

    let (escrow_pda, _bump) = Pubkey::find_program_address(
        &[b"escrow", maker.pubkey().as_ref(), &seed.to_le_bytes()],
        &escrow::id(),
    );

    let vault = get_associated_token_address(&escrow_pda, &mint_a.pubkey());

    let make_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Make {
            maker: maker.pubkey(),
            escrow: escrow_pda,
            mint_a: mint_a.pubkey(),
            mint_b: mint_b.pubkey(),
            maker_ata_a,
            vault,
            associated_token_program: ATA_PROGRAM_ID,
            token_program: spl_token::id(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Make {
            seed,
            receive,
            amount,
        }
        .data(),
    };

    send(svm, &[make_ix], maker, &[maker]);

    EscrowCtx {
        escrow_pda,
        vault,
        maker_ata_a,
        mint_a,
        mint_b,
        seed,
        receive,
        amount,
        maker_initial,
    }
}

fn account_closed(svm: &LiteSVM, key: &Pubkey) -> bool {
    match svm.get_account(key) {
        None => true,
        Some(acc) => acc.lamports == 0 || acc.data.is_empty(),
    }
}

#[test]
fn test_make() {
    let (mut svm, maker) = setup();
    let ctx = setup_with_make(&mut svm, &maker);

    let escrow_acc = svm.get_account(&ctx.escrow_pda).expect("escrow not created");
    let escrow_state = Escrow::try_deserialize(&mut escrow_acc.data.as_slice()).unwrap();
    assert_eq!(escrow_state.seed, ctx.seed);
    assert_eq!(escrow_state.receive, ctx.receive);
    assert_eq!(escrow_state.maker, maker.pubkey());
    assert_eq!(escrow_state.mint_a, ctx.mint_a.pubkey());
    assert_eq!(escrow_state.mint_b, ctx.mint_b.pubkey());

    let vault_acc = svm.get_account(&ctx.vault).expect("vault not created");
    let vault_state = TokenAccount::try_deserialize(&mut vault_acc.data.as_slice()).unwrap();
    assert_eq!(vault_state.amount, ctx.amount, "vault balance mismatch");
    assert_eq!(vault_state.mint, ctx.mint_a.pubkey());
    assert_eq!(vault_state.owner, ctx.escrow_pda);

    let maker_ata_acc = svm.get_account(&ctx.maker_ata_a).unwrap();
    let maker_ata_state =
        TokenAccount::try_deserialize(&mut maker_ata_acc.data.as_slice()).unwrap();
    assert_eq!(maker_ata_state.amount, ctx.maker_initial - ctx.amount);
}

#[test]
fn test_refund() {
    let (mut svm, maker) = setup();
    let ctx = setup_with_make(&mut svm, &maker);

    let refund_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Refund {
            maker: maker.pubkey(),
            escrow: ctx.escrow_pda,
            mint_a: ctx.mint_a.pubkey(),
            mint_b: ctx.mint_b.pubkey(),
            maker_ata_a: ctx.maker_ata_a,
            vault: ctx.vault,
            associated_token_program: ATA_PROGRAM_ID,
            token_program: spl_token::id(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Refund {}.data(),
    };

    send(&mut svm, &[refund_ix], &maker, &[&maker]);

    assert!(account_closed(&svm, &ctx.escrow_pda), "escrow not closed");
    assert!(account_closed(&svm, &ctx.vault), "vault not closed");

    let maker_ata_acc = svm.get_account(&ctx.maker_ata_a).unwrap();
    let maker_ata_state =
        TokenAccount::try_deserialize(&mut maker_ata_acc.data.as_slice()).unwrap();
    assert_eq!(
        maker_ata_state.amount, ctx.maker_initial,
        "maker did not get full refund"
    );
}

#[test]
fn test_take() {
    let (mut svm, maker) = setup();
    let ctx = setup_with_make(&mut svm, &maker);

    let taker = Keypair::new();
    svm.airdrop(&taker.pubkey(), 10_000_000_000).unwrap();

    let taker_initial_b: u64 = 5_000_000;
    let taker_ata_b = create_ata_and_mint_to(
        &mut svm,
        &maker,
        &ctx.mint_b.pubkey(),
        &taker.pubkey(),
        taker_initial_b,
    );

    let taker_ata_a = get_associated_token_address(&taker.pubkey(), &ctx.mint_a.pubkey());
    let maker_ata_b = get_associated_token_address(&maker.pubkey(), &ctx.mint_b.pubkey());

    let take_ix = Instruction {
        program_id: escrow::id(),
        accounts: escrow::accounts::Take {
            taker: taker.pubkey(),
            maker: maker.pubkey(),
            escrow: ctx.escrow_pda,
            mint_a: ctx.mint_a.pubkey(),
            mint_b: ctx.mint_b.pubkey(),
            vault: ctx.vault,
            taker_ata_a,
            taker_ata_b,
            maker_ata_b,
            associated_token_program: ATA_PROGRAM_ID,
            token_program: spl_token::id(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: escrow::instruction::Take {}.data(),
    };

    send(&mut svm, &[take_ix], &taker, &[&taker]);

    assert!(account_closed(&svm, &ctx.escrow_pda), "escrow not closed");
    assert!(account_closed(&svm, &ctx.vault), "vault not closed");

    let taker_a_acc = svm.get_account(&taker_ata_a).expect("taker_ata_a missing");
    let taker_a_state = TokenAccount::try_deserialize(&mut taker_a_acc.data.as_slice()).unwrap();
    assert_eq!(taker_a_state.amount, ctx.amount, "taker did not get mint_a");

    let maker_b_acc = svm.get_account(&maker_ata_b).expect("maker_ata_b missing");
    let maker_b_state = TokenAccount::try_deserialize(&mut maker_b_acc.data.as_slice()).unwrap();
    assert_eq!(maker_b_state.amount, ctx.receive, "maker did not get mint_b");

    let taker_b_acc = svm.get_account(&taker_ata_b).unwrap();
    let taker_b_state = TokenAccount::try_deserialize(&mut taker_b_acc.data.as_slice()).unwrap();
    assert_eq!(
        taker_b_state.amount,
        taker_initial_b - ctx.receive,
        "taker mint_b balance wrong"
    );
}
