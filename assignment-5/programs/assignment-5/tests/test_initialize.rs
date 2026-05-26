mod utils;

use {
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_keypair::Signer,
    solana_message::{v0, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_transaction::versioned::VersionedTransaction,
};
use utils::*;

#[test]
fn test_initialize_basic() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let seed: u64 = 42;
    let fee: u16 = 30;

    let mint_x = create_mint(&mut svm, &payer);
    let mint_y = create_mint(&mut svm, &payer);

    let (config_pda, _config_bump) = Pubkey::find_program_address(
        &[b"config".as_ref(), seed.to_le_bytes().as_ref()],
        &program_id,
    );

    let (mint_lp_pda, _lp_bump) = Pubkey::find_program_address(
        &[b"lp".as_ref(), config_pda.as_ref()],
        &program_id,
    );

    let vault_x = get_associated_token_address(&config_pda, &mint_x);
    let vault_y = get_associated_token_address(&config_pda, &mint_y);

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Initialize {
            seed,
            fee,
            authority: None,
        }
        .data(),
        assignment_5::accounts::Initialize {
            initializer: payer.pubkey(),
            mint_x,
            mint_y,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            config: config_pda,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = v0::Message::try_compile(&payer.pubkey(), &[ix], &[], blockhash).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "Initialize failed: {:?}", res.err());

    let config_acc = svm.get_account(&config_pda).expect("Config account should exist");
    assert!(config_acc.data.len() > 0);
}

#[test]
fn test_initialize_with_authority() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let seed: u64 = 99;
    let fee: u16 = 100;
    let authority = Some(payer.pubkey());

    let mint_x = create_mint(&mut svm, &payer);
    let mint_y = create_mint(&mut svm, &payer);

    let (config_pda, _bump) = Pubkey::find_program_address(
        &[b"config".as_ref(), seed.to_le_bytes().as_ref()],
        &program_id,
    );

    let (mint_lp_pda, _lp_bump) = Pubkey::find_program_address(
        &[b"lp".as_ref(), config_pda.as_ref()],
        &program_id,
    );

    let vault_x = get_associated_token_address(&config_pda, &mint_x);
    let vault_y = get_associated_token_address(&config_pda, &mint_y);

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Initialize {
            seed,
            fee,
            authority,
        }
        .data(),
        assignment_5::accounts::Initialize {
            initializer: payer.pubkey(),
            mint_x,
            mint_y,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            config: config_pda,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = v0::Message::try_compile(&payer.pubkey(), &[ix], &[], blockhash).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "Initialize with authority failed: {:?}", res.err());
}

#[test]
fn test_initialize_same_seed_fails() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let seed: u64 = 7;
    let fee: u16 = 0;

    let mint_x = create_mint(&mut svm, &payer);
    let mint_y = create_mint(&mut svm, &payer);

    let (config_pda, _bump) = Pubkey::find_program_address(
        &[b"config".as_ref(), seed.to_le_bytes().as_ref()],
        &program_id,
    );

    let (mint_lp_pda, _lp_bump) = Pubkey::find_program_address(
        &[b"lp".as_ref(), config_pda.as_ref()],
        &program_id,
    );

    let vault_x = get_associated_token_address(&config_pda, &mint_x);
    let vault_y = get_associated_token_address(&config_pda, &mint_y);

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Initialize {
            seed,
            fee,
            authority: None,
        }
        .data(),
        assignment_5::accounts::Initialize {
            initializer: payer.pubkey(),
            mint_x,
            mint_y,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            config: config_pda,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = v0::Message::try_compile(&payer.pubkey(), &[ix], &[], blockhash).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "First init should succeed");

    let mint_x2 = create_mint(&mut svm, &payer);
    let mint_y2 = create_mint(&mut svm, &payer);

    let vault_x2 = get_associated_token_address(&config_pda, &mint_x2);
    let vault_y2 = get_associated_token_address(&config_pda, &mint_y2);

    let ix2 = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Initialize {
            seed,
            fee,
            authority: None,
        }
        .data(),
        assignment_5::accounts::Initialize {
            initializer: payer.pubkey(),
            mint_x: mint_x2,
            mint_y: mint_y2,
            mint_lp: mint_lp_pda,
            vault_x: vault_x2,
            vault_y: vault_y2,
            config: config_pda,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash2 = svm.latest_blockhash();
    let msg2 = v0::Message::try_compile(&payer.pubkey(), &[ix2], &[], blockhash2).unwrap();
    let tx2 = VersionedTransaction::try_new(VersionedMessage::V0(msg2), &[&payer]).unwrap();

    let res2 = svm.send_transaction(tx2);
    assert!(res2.is_err(), "Re-initializing same seed should fail");
}
