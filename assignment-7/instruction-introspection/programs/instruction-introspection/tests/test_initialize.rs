//! End-to-end tests for the AMM, run on LiteSVM against the compiled program.
//!
//! Prerequisite: build the program first so the `.so` exists:
//!     anchor build      (or: cargo build-sbf)
//! then:
//!     cargo test -p instruction-introspection

use {
    anchor_lang::{
        solana_program::{
            instruction::Instruction, system_instruction, system_program,
        },
        InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{spl_associated_token_account, ID as ATA_PROGRAM_ID},
        token::{spl_token, ID as TOKEN_PROGRAM_ID},
    },
    
    instruction_introspection::{Side, CONFIG_SEED, LP_SEED},
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    spl_token::state::{Account as SplAccount, Mint as SplMint},
};
use anchor_spl::associated_token::get_associated_token_address;
use solana_instructions_sysvar::{
    load_current_index_checked, load_instruction_at_checked, ID as INSTRUCTIONS_SYSVAR_ID,
};
use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::solana_program::pubkey::Pubkey;

const POOL_SEED: u64 = 1;
const FEE_BPS: u16 = 30; // 0.30%
const MINT_DECIMALS: u8 = 6;
const USER_FUNDING: u64 = 10_000_000;

// ---------- shared harness ----------

struct Pool {
    svm: LiteSVM,
    payer: Keypair,
    mint_a: Pubkey,
    mint_b: Pubkey,
    config: Pubkey,
    mint_lp: Pubkey,
    vault_a: Pubkey,
    vault_b: Pubkey,
    user_a: Pubkey,
    user_b: Pubkey,
    user_lp: Pubkey,
}

fn load_svm() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!("../../../target/deploy/instruction_introspection.so");
    svm.add_program(instruction_introspection::ID, bytes).unwrap();
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
    (svm, payer)
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra_signers: &[&Keypair],
    ixs: &[Instruction],
) -> Result<(), litesvm::types::FailedTransactionMetadata> {
    let bh = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &bh);
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra_signers);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    svm.send_transaction(tx).map(|_| ())
}

fn create_mint(svm: &mut LiteSVM, payer: &Keypair) -> Pubkey {
    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(SplMint::LEN);
    let create = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        SplMint::LEN as u64,
        &TOKEN_PROGRAM_ID,
    );
    let init = spl_token::instruction::initialize_mint2(
        &TOKEN_PROGRAM_ID,
        &mint.pubkey(),
        &payer.pubkey(), // mint authority = payer
        None,
        MINT_DECIMALS,
    )
    .unwrap();
    send(svm, payer, &[&mint], &[create, init]).unwrap();
    mint.pubkey()
}

fn create_ata(svm: &mut LiteSVM, payer: &Keypair, owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        owner,
        mint,
        &TOKEN_PROGRAM_ID,
    );
    send(svm, payer, &[], &[ix]).unwrap();
    ata
}

fn mint_to(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey, dest: &Pubkey, amount: u64) {
    let ix = spl_token::instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        mint,
        dest,
        &payer.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    send(svm, payer, &[], &[ix]).unwrap();
}

fn token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let acc = svm.get_account(ata).expect("token account exists");
    SplAccount::unpack(&acc.data).unwrap().amount
}

/// initialize + first deposit, leaving an even 1:1 pool funded with `liquidity` each side.
fn setup_pool(liquidity: u64) -> Pool {
    let (mut svm, payer) = load_svm();
    let program_id = instruction_introspection::ID;

    // Two mints, sorted so mint_a < mint_b (the program enforces this order).
    let (mut m1, mut m2) = (create_mint(&mut svm, &payer), create_mint(&mut svm, &payer));
    if m1 > m2 {
        std::mem::swap(&mut m1, &mut m2);
    }
    let (mint_a, mint_b) = (m1, m2);

    let (config, _) =
        Pubkey::find_program_address(&[CONFIG_SEED, &POOL_SEED.to_le_bytes()], &program_id);
    let (mint_lp, _) = Pubkey::find_program_address(&[LP_SEED, config.as_ref()], &program_id);

    let vault_a = get_associated_token_address(&config, &mint_a);
    let vault_b = get_associated_token_address(&config, &mint_b);

    // initialize
    let init_ix = Instruction {
        program_id,
        accounts: instruction_introspection::accounts::Initialize {
            authority: payer.pubkey(),
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            config,
            mint_lp,
            system_program: system_program::ID,
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ATA_PROGRAM_ID,
            token_program_lp: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: instruction_introspection::instruction::Initialize {
            seed: POOL_SEED,
            fee: FEE_BPS,
            authority: None,
        }
        .data(),
    };
    send(&mut svm, &payer, &[], &[init_ix]).expect("initialize");

    // user token accounts + funding
    let user = payer.pubkey();
    let user_a = create_ata(&mut svm, &payer, &user, &mint_a);
    let user_b = create_ata(&mut svm, &payer, &user, &mint_b);
    let user_lp = get_associated_token_address(&user, &mint_lp); // created by deposit (init_if_needed)
    mint_to(&mut svm, &payer, &mint_a, &user_a, USER_FUNDING);
    mint_to(&mut svm, &payer, &mint_b, &user_b, USER_FUNDING);

    // first deposit: mints `liquidity` LP, pulls `liquidity` of each token
    let dep_ix = Instruction {
        program_id,
        accounts: instruction_introspection::accounts::Deposit {
            user,
            mint_a,
            mint_b,
            vault_a,
            vault_b,
            config,
            mint_lp,
            user_a,
            user_b,
            user_lp,
            token_program: TOKEN_PROGRAM_ID,
            token_program_lp: TOKEN_PROGRAM_ID,
            associated_token_program: ATA_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: instruction_introspection::instruction::Deposit {
            amount: liquidity,
            max_a: liquidity,
            max_b: liquidity,
        }
        .data(),
    };
    send(&mut svm, &payer, &[], &[dep_ix]).expect("deposit");

    Pool {
        svm,
        payer,
        mint_a,
        mint_b,
        config,
        mint_lp,
        vault_a,
        vault_b,
        user_a,
        user_b,
        user_lp,
    }
}

// ---------- tests ----------

#[test]
fn test_initialize() {
    // setup_pool runs initialize (and deposit); reaching here without panic = success.
    let pool = setup_pool(1_000_000);
    // config account must now exist and be owned by the program.
    let cfg = pool.svm.get_account(&pool.config).expect("config created");
    assert_eq!(cfg.owner, instruction_introspection::ID);
}

#[test]
fn test_deposit() {
    let liquidity = 1_000_000u64;
    let pool = setup_pool(liquidity);

    // Vaults hold the deposited liquidity, user holds LP.
    assert_eq!(token_balance(&pool.svm, &pool.vault_a), liquidity);
    assert_eq!(token_balance(&pool.svm, &pool.vault_b), liquidity);
    assert_eq!(token_balance(&pool.svm, &pool.user_lp), liquidity);
    assert_eq!(
        token_balance(&pool.svm, &pool.user_a),
        USER_FUNDING - liquidity
    );
}

#[test]
fn test_swap() {
    let mut pool = setup_pool(1_000_000);
    let program_id = instruction_introspection::ID;

    let before_b = token_balance(&pool.svm, &pool.user_b);
    let amount_in = 100_000u64;

    let swap_ix = Instruction {
        program_id,
        accounts: instruction_introspection::accounts::Swap {
            user: pool.payer.pubkey(),
            mint_a: pool.mint_a,
            mint_b: pool.mint_b,
            vault_a: pool.vault_a,
            vault_b: pool.vault_b,
            config: pool.config,
            user_a: pool.user_a,
            user_b: pool.user_b,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: instruction_introspection::instruction::Swap {
            side: Side::a, // give A, get B
            amount_in,
            min_out: 1,
        }
        .data(),
    };
    let payer = pool.payer.insecure_clone();
    send(&mut pool.svm, &payer, &[], &[swap_ix]).expect("swap");

    // Paid A in, received B out.
    assert_eq!(token_balance(&pool.svm, &pool.vault_a), 1_000_000 + amount_in);
    let after_b = token_balance(&pool.svm, &pool.user_b);
    assert!(after_b > before_b, "user should receive token B");
    // const-product + 0.3% fee: out is strictly less than input on a 1:1 pool.
    assert!(after_b - before_b < amount_in);
}

#[test]
fn test_withdraw_with_prior_burn() {
    let mut pool = setup_pool(1_000_000);
    let payer = pool.payer.insecure_clone();
    let user = payer.pubkey();
    let program_id = instruction_introspection::ID;

    let burn_amount = 500_000u64;
    let before_a = token_balance(&pool.svm, &pool.user_a);
    let before_b = token_balance(&pool.svm, &pool.user_b);

    // ix[0]: SPL Token burn of LP (the instruction withdraw introspects).
    let burn_ix = spl_token::instruction::burn(
        &TOKEN_PROGRAM_ID,
        &pool.user_lp,
        &pool.mint_lp,
        &user,
        &[],
        burn_amount,
    )
    .unwrap();

    // ix[1]: withdraw — reads ix[0], pays out proportional share.
    let withdraw_ix = Instruction {
        program_id,
        accounts: instruction_introspection::accounts::Withdraw {
            user,
            mint_a: pool.mint_a,
            mint_b: pool.mint_b,
            vault_a: pool.vault_a,
            vault_b: pool.vault_b,
            config: pool.config,
            mint_lp: pool.mint_lp,
            user_a: pool.user_a,
            user_b: pool.user_b,
            instructions_sysvar: INSTRUCTIONS_SYSVAR_ID,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: instruction_introspection::instruction::Withdraw { min_a: 1, min_b: 1 }.data(),
    };

    send(&mut pool.svm, &payer, &[], &[burn_ix, withdraw_ix]).expect("burn + withdraw");

    // Burned 50% of a 1:1 pool of 1_000_000 → 500_000 of each back.
    assert_eq!(token_balance(&pool.svm, &pool.user_a), before_a + 500_000);
    assert_eq!(token_balance(&pool.svm, &pool.user_b), before_b + 500_000);
    assert_eq!(token_balance(&pool.svm, &pool.user_lp), 1_000_000 - burn_amount);
}

#[test]
fn test_withdraw_without_burn_fails() {
    let mut pool = setup_pool(1_000_000);
    let payer = pool.payer.insecure_clone();
    let user = payer.pubkey();
    let program_id = instruction_introspection::ID;

    // withdraw alone (no preceding burn) must fail the introspection check.
    let withdraw_ix = Instruction {
        program_id,
        accounts: instruction_introspection::accounts::Withdraw {
            user,
            mint_a: pool.mint_a,
            mint_b: pool.mint_b,
            vault_a: pool.vault_a,
            vault_b: pool.vault_b,
            config: pool.config,
            mint_lp: pool.mint_lp,
            user_a: pool.user_a,
            user_b: pool.user_b,
            instructions_sysvar: INSTRUCTIONS_SYSVAR_ID,
            token_program: TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: instruction_introspection::instruction::Withdraw { min_a: 0, min_b: 0 }.data(),
    };

    let res = send(&mut pool.svm, &payer, &[], &[withdraw_ix]);
    assert!(res.is_err(), "withdraw without prior burn should fail");
}
