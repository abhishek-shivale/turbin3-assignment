mod utils;

use {
    anchor_lang::{InstructionData, ToAccountMetas},
    solana_keypair::Signer,
    solana_message::{v0, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_transaction::versioned::VersionedTransaction,
};
use utils::*;

fn setup_pool_with_liquidity(
    svm: &mut litesvm::LiteSVM,
    payer: &solana_keypair::Keypair,
) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
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

    let user_x = create_user_ata(svm, payer, &mint_x, &payer.pubkey());
    let user_y = create_user_ata(svm, payer, &mint_y, &payer.pubkey());

    let deposit_x: u64 = 200000;
    let deposit_y: u64 = 200000;
    let lp_amount: u64 = 100000;

    mint_tokens(svm, payer, &mint_x, &user_x, deposit_x, payer);
    mint_tokens(svm, payer, &mint_y, &user_y, deposit_y, payer);

    let user_lp = get_associated_token_address(&payer.pubkey(), &mint_lp_pda);

    let ix2 = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
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

    let blockhash2 = svm.latest_blockhash();
    let msg2 = v0::Message::try_compile(&payer.pubkey(), &[ix2], &[], blockhash2).unwrap();
    let tx2 = VersionedTransaction::try_new(VersionedMessage::V0(msg2), &[payer]).unwrap();
    svm.send_transaction(tx2).unwrap();

    (
        config_pda,
        mint_lp_pda,
        mint_x,
        mint_y,
        vault_x,
        vault_y,
        user_x,
        user_y,
        user_lp,
    )
}

#[test]
fn test_withdraw_partial() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y, user_x, user_y, user_lp) =
        setup_pool_with_liquidity(&mut svm, &payer);

    let user_lp_balance_before = {
        let acc = svm.get_account(&user_lp).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };

    let vault_x_before = {
        let acc = svm.get_account(&vault_x).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };
    let vault_y_before = {
        let acc = svm.get_account(&vault_y).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };

    let withdraw_lp: u64 = 30000;

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Withdraw {
            amount: withdraw_lp,
            min_x: 1,
            min_y: 1,
        }
        .data(),
        assignment_5::accounts::Withdraw {
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
    assert!(res.is_ok(), "Withdraw failed: {:?}", res.err());

    let user_lp_balance_after = {
        let acc = svm.get_account(&user_lp).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };

    let user_x_after = {
        let acc = svm.get_account(&user_x).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };
    let user_y_after = {
        let acc = svm.get_account(&user_y).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };

    assert_eq!(
        user_lp_balance_after,
        user_lp_balance_before - withdraw_lp,
        "LP tokens should be burned"
    );

    let vault_x_after = {
        let acc = svm.get_account(&vault_x).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };
    let vault_y_after = {
        let acc = svm.get_account(&vault_y).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };

    assert!(vault_x_after < vault_x_before, "Vault X should have less tokens");
    assert!(vault_y_after < vault_y_before, "Vault Y should have less tokens");
    assert!(user_x_after > 0, "User should receive X tokens");
    assert!(user_y_after > 0, "User should receive Y tokens");
}

#[test]
fn test_withdraw_full() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y, user_x, user_y, user_lp) =
        setup_pool_with_liquidity(&mut svm, &payer);

    let user_lp_balance = {
        let acc = svm.get_account(&user_lp).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Withdraw {
            amount: user_lp_balance,
            min_x: 1,
            min_y: 1,
        }
        .data(),
        assignment_5::accounts::Withdraw {
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
    assert!(res.is_ok(), "Full withdraw failed: {:?}", res.err());

    let user_lp_after = {
        let acc = svm.get_account(&user_lp).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };
    assert_eq!(user_lp_after, 0, "All LP tokens should be burned");
}

#[test]
fn test_withdraw_slippage_exceeded() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y, user_x, user_y, user_lp) =
        setup_pool_with_liquidity(&mut svm, &payer);

    let user_lp_balance = {
        let acc = svm.get_account(&user_lp).unwrap();
        u64::from_le_bytes(acc.data[64..72].try_into().unwrap())
    };

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Withdraw {
            amount: user_lp_balance,
            min_x: u64::MAX,
            min_y: u64::MAX,
        }
        .data(),
        assignment_5::accounts::Withdraw {
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
    assert!(res.is_err(), "Withdraw with high min should fail due to slippage");
}

#[test]
fn test_withdraw_zero_amount_fails() {
    let (mut svm, payer) = setup();
    let program_id = assignment_5::id();

    let (config_pda, mint_lp_pda, mint_x, mint_y, vault_x, vault_y, user_x, user_y, user_lp) =
        setup_pool_with_liquidity(&mut svm, &payer);

    let ix = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &assignment_5::instruction::Withdraw {
            amount: 0,
            min_x: 1,
            min_y: 1,
        }
        .data(),
        assignment_5::accounts::Withdraw {
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
    assert!(res.is_err(), "Withdraw with zero amount should fail");
}
