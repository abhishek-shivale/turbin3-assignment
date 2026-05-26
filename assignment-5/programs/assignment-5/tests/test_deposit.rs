mod utils;

use {
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_keypair::Signer,
    solana_keypair::Keypair,
    solana_message::{v0, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_transaction::versioned::VersionedTransaction,
};
use utils::*;

fn initialize_pool(svm: &mut litesvm::LiteSVM, payer: &solana_keypair::Keypair) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    let program_id = assignment_5::id();
    let seed: u64 = 1;
    let fee: u16 = 30;

    let mint_x = create_mint(svm, payer);
    let mint_y = create_mint(svm, payer);

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
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[payer]).unwrap();
    svm.send_transaction(tx).unwrap();

    (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y)
}

#[test]
fn test_deposit_first_deposit() {
    let (mut svm, payer) = setup();

    let (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y) =
        initialize_pool(&mut svm, &payer);

    let user_x = create_user_ata(&mut svm, &payer, &mint_x, &payer.pubkey());
    let user_y = create_user_ata(&mut svm, &payer, &mint_y, &payer.pubkey());

    let deposit_x: u64 = 10000;
    let deposit_y: u64 = 20000;
    let lp_amount: u64 = 50000;

    mint_tokens(&mut svm, &payer, &mint_x, &user_x, deposit_x, &payer);
    mint_tokens(&mut svm, &payer, &mint_y, &user_y, deposit_y, &payer);

    let user_lp = get_associated_token_address(&payer.pubkey(), &mint_lp_pda);

    let program_id = assignment_5::id();
    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Deposit {
            amount: lp_amount,
            max_x: deposit_x,
            max_y: deposit_y,
        }
        .data(),
        assignment_5::accounts::Deposit {
            user: payer.pubkey(),
            mint_x,
            mint_y,
            config: config_pda,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = v0::Message::try_compile(&payer.pubkey(), &[ix], &[], blockhash).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok(), "First deposit failed: {:?}", res.err());

    let vault_x_data = svm.get_account(&vault_x).unwrap();
    let vault_x_amount =
        u64::from_le_bytes(vault_x_data.data[64..72].try_into().unwrap());
    let vault_y_data = svm.get_account(&vault_y).unwrap();
    let vault_y_amount =
        u64::from_le_bytes(vault_y_data.data[64..72].try_into().unwrap());

    assert_eq!(vault_x_amount, deposit_x);
    assert_eq!(vault_y_amount, deposit_y);

    let user_lp_data = svm.get_account(&user_lp).unwrap();
    let user_lp_amount =
        u64::from_le_bytes(user_lp_data.data[64..72].try_into().unwrap());
    assert_eq!(user_lp_amount, lp_amount);
}

#[test]
fn test_deposit_second_deposit() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y) =
        initialize_pool(&mut svm, &payer);

    let user_x = create_user_ata(&mut svm, &payer, &mint_x, &payer.pubkey());
    let user_y = create_user_ata(&mut svm, &payer, &mint_y, &payer.pubkey());

    let initial_x: u64 = 100000;
    let initial_y: u64 = 200000;
    let first_lp: u64 = 100000;

    mint_tokens(&mut svm, &payer, &mint_x, &user_x, initial_x, &payer);
    mint_tokens(&mut svm, &payer, &mint_y, &user_y, initial_y, &payer);

    let user_lp = get_associated_token_address(&payer.pubkey(), &mint_lp_pda);

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Deposit {
            amount: first_lp,
            max_x: initial_x,
            max_y: initial_y,
        }
        .data(),
        assignment_5::accounts::Deposit {
            user: payer.pubkey(),
            mint_x,
            mint_y,
            config: config_pda,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = v0::Message::try_compile(&payer.pubkey(), &[ix], &[], blockhash).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer]).unwrap();
    svm.send_transaction(tx).unwrap();

    let second_deposit_x: u64 = 50000;
    let second_deposit_y: u64 = 100000;
    mint_tokens(&mut svm, &payer, &mint_x, &user_x, second_deposit_x, &payer);
    mint_tokens(&mut svm, &payer, &mint_y, &user_y, second_deposit_y, &payer);

    let second_lp: u64 = 50000;
    let ix2 = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Deposit {
            amount: second_lp,
            max_x: second_deposit_x,
            max_y: second_deposit_y,
        }
        .data(),
        assignment_5::accounts::Deposit {
            user: payer.pubkey(),
            mint_x,
            mint_y,
            config: config_pda,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        }
        .to_account_metas(None),
    );

    let blockhash2 = svm.latest_blockhash();
    let msg2 = v0::Message::try_compile(&payer.pubkey(), &[ix2], &[], blockhash2).unwrap();
    let tx2 = VersionedTransaction::try_new(VersionedMessage::V0(msg2), &[&payer]).unwrap();

    let res2 = svm.send_transaction(tx2);
    assert!(res2.is_ok(), "Second deposit failed: {:?}", res2.err());

    let vault_x_data = svm.get_account(&vault_x).unwrap();
    let vault_x_amount =
        u64::from_le_bytes(vault_x_data.data[64..72].try_into().unwrap());
    let vault_y_data = svm.get_account(&vault_y).unwrap();
    let vault_y_amount =
        u64::from_le_bytes(vault_y_data.data[64..72].try_into().unwrap());

    assert!(vault_x_amount > initial_x);
    assert!(vault_y_amount > initial_y);
    assert!(vault_x_amount <= initial_x + second_deposit_x);
    assert!(vault_y_amount <= initial_y + second_deposit_y);

    let user_lp_data = svm.get_account(&user_lp).unwrap();
    let user_lp_amount =
        u64::from_le_bytes(user_lp_data.data[64..72].try_into().unwrap());
    assert_eq!(user_lp_amount, first_lp + second_lp);
}

#[test]
fn test_deposit_slippage_exceeded() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y) =
        initialize_pool(&mut svm, &payer);

    let user_x = create_user_ata(&mut svm, &payer, &mint_x, &payer.pubkey());
    let user_y = create_user_ata(&mut svm, &payer, &mint_y, &payer.pubkey());

    let initial_x: u64 = 100000;
    let initial_y: u64 = 200000;
    let first_lp: u64 = 100000;

    mint_tokens(&mut svm, &payer, &mint_x, &user_x, initial_x, &payer);
    mint_tokens(&mut svm, &payer, &mint_y, &user_y, initial_y, &payer);

    let user_lp = get_associated_token_address(&payer.pubkey(), &mint_lp_pda);

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Deposit {
            amount: first_lp,
            max_x: initial_x,
            max_y: initial_y,
        }
        .data(),
        assignment_5::accounts::Deposit {
            user: payer.pubkey(),
            mint_x,
            mint_y,
            config: config_pda,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = v0::Message::try_compile(&payer.pubkey(), &[ix], &[], blockhash).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer]).unwrap();
    svm.send_transaction(tx).unwrap();

    let more_x: u64 = 50000;
    let more_y: u64 = 50000;
    mint_tokens(&mut svm, &payer, &mint_x, &user_x, more_x, &payer);
    mint_tokens(&mut svm, &payer, &mint_y, &user_y, more_y, &payer);

    let ix2 = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Deposit {
            amount: 100000,
            max_x: 1,
            max_y: 1,
        }
        .data(),
        assignment_5::accounts::Deposit {
            user: payer.pubkey(),
            mint_x,
            mint_y,
            config: config_pda,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        }
        .to_account_metas(None),
    );

    let blockhash2 = svm.latest_blockhash();
    let msg2 = v0::Message::try_compile(&payer.pubkey(), &[ix2], &[], blockhash2).unwrap();
    let tx2 = VersionedTransaction::try_new(VersionedMessage::V0(msg2), &[&payer]).unwrap();

    let res2 = svm.send_transaction(tx2);
    assert!(res2.is_err(), "Deposit with low max should fail due to slippage");
}

#[test]
fn test_deposit_zero_amount_fails() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y) =
        initialize_pool(&mut svm, &payer);

    let user_x = create_user_ata(&mut svm, &payer, &mint_x, &payer.pubkey());
    let user_y = create_user_ata(&mut svm, &payer, &mint_y, &payer.pubkey());

    let initial_x: u64 = 100000;
    let initial_y: u64 = 200000;
    let first_lp: u64 = 100000;

    mint_tokens(&mut svm, &payer, &mint_x, &user_x, initial_x, &payer);
    mint_tokens(&mut svm, &payer, &mint_y, &user_y, initial_y, &payer);

    let user_lp = get_associated_token_address(&payer.pubkey(), &mint_lp_pda);

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Deposit {
            amount: first_lp,
            max_x: initial_x,
            max_y: initial_y,
        }
        .data(),
        assignment_5::accounts::Deposit {
            user: payer.pubkey(),
            mint_x,
            mint_y,
            config: config_pda,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = v0::Message::try_compile(&payer.pubkey(), &[ix], &[], blockhash).unwrap();
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(msg), &[&payer]).unwrap();
    svm.send_transaction(tx).unwrap();

    let ix2 = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Deposit {
            amount: 0,
            max_x: 100000,
            max_y: 100000,
        }
        .data(),
        assignment_5::accounts::Deposit {
            user: payer.pubkey(),
            mint_x,
            mint_y,
            config: config_pda,
            mint_lp: mint_lp_pda,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
            associated_token_program: anchor_spl::associated_token::ID,
        }
        .to_account_metas(None),
    );

    let blockhash2 = svm.latest_blockhash();
    let msg2 = v0::Message::try_compile(&payer.pubkey(), &[ix2], &[], blockhash2).unwrap();
    let tx2 = VersionedTransaction::try_new(VersionedMessage::V0(msg2), &[&payer]).unwrap();

    let res2 = svm.send_transaction(tx2);
    assert!(res2.is_err(), "Deposit with zero amount should fail");
}
