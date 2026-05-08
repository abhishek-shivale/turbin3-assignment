use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction::create_account;
use spl_associated_token_account_interface::{
    address::get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token_interface::{
    id as token_program_id,
    instruction::{initialize_account, initialize_mint, mint_to},
    state::{Account, Mint},
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = RpcClient::new_with_commitment(
        String::from("http://localhost:8899"),
        CommitmentConfig::confirmed(),
    );

    let key_pair = Keypair::new();

    let mint = Keypair::new();

    let airdrop_sig = client
        .request_airdrop(&key_pair.pubkey(), LAMPORTS_PER_SOL * 10)
        .await?;

    loop {
        let confirmed = client.confirm_transaction(&airdrop_sig).await?;
        if confirmed {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let mint_rent = client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await?;

    let latest_blockhash = client.get_latest_blockhash().await?;
    // mint token account - its token account where metadata of token lives

    let transaction = Transaction::new_signed_with_payer(
        &[
            create_account(
                &key_pair.pubkey(),
                &mint.pubkey(),
                mint_rent,
                Mint::LEN as u64,
                &token_program_id(),
            ),
            initialize_mint(
                &token_program_id(),
                &mint.pubkey(),
                &key_pair.pubkey(),
                Some(&key_pair.pubkey()),
                3,
            )?,
        ],
        Some(&key_pair.pubkey()),
        &[&key_pair, &mint],
        latest_blockhash,
    );

    let transaction_signature = client.send_and_confirm_transaction(&transaction).await?;
    let mint_account = client.get_account(&mint.pubkey()).await?;
    let mint_data = Mint::unpack(&mint_account.data)?;
    let latest_blockhash = client.get_latest_blockhash().await?;

    // token - ATA - here the token amount lives (global supply balance)

    let transaction_ata = Transaction::new_signed_with_payer(
        &[create_associated_token_account(
            &key_pair.pubkey(),
            &key_pair.pubkey(),
            &mint.pubkey(),
            &token_program_id(),
        )],
        Some(&key_pair.pubkey()),
        &[&key_pair],
        latest_blockhash,
    );

    let transaction_signature_ata = client
        .send_and_confirm_transaction(&transaction_ata)
        .await?;

    let ata_token_account = get_associated_token_address(&key_pair.pubkey(), &mint.pubkey());

    let token_account = client.get_account(&ata_token_account).await?;

    // let token_data = Account::unpack(&token_account.data)?;
    //
    // let token_account = Keypair::new();
    //
    // let toke_account_rent = client
    //     .get_minimum_balance_for_rent_exemption(Account::LEN)
    //     .await?;
    //
    // let latest_blockhash = client.get_latest_blockhash().await?;
    //
    // // 
    //
    // let transaction_ta = Transaction::new_signed_with_payer(
    //     &[
    //         create_account(
    //             &key_pair.pubkey(),
    //             &token_account.pubkey(),
    //             toke_account_rent,
    //             Account::LEN as u64,
    //             &token_program_id(),
    //         ),
    //         initialize_account(
    //             &token_program_id(),
    //             &token_account.pubkey(),
    //             &mint.pubkey(),
    //             &key_pair.pubkey(),
    //         )?,
    //     ],
    //     Some(&key_pair.pubkey()),
    //     &[&key_pair, &token_account],
    //     latest_blockhash,
    // );
    // let transaction_signature_ta = client.send_and_confirm_transaction(&transaction_ta).await?;
    // let token_account_data_ta = client.get_account(&token_account.pubkey()).await?;
    // let token_data_ta = Account::unpack(&token_account_data_ta.data)?;

    let latest_blockhash = client.get_latest_blockhash().await?;

    let mint_tx = Transaction::new_signed_with_payer(
        &[mint_to(
            &token_program_id(),
            &mint.pubkey(),
            &ata_token_account,
            &key_pair.pubkey(),
            &[],
            1_000_000,
        )?],
        Some(&key_pair.pubkey()),
        &[&key_pair],
        latest_blockhash,
    );
    let sig = client.send_and_confirm_transaction(&mint_tx).await?;

    let updated = client.get_account(&ata_token_account).await?;
    let updated_data = Account::unpack(&updated.data)?;

    println!("Mint Address: {}", mint.pubkey());
    println!("Mint Account: {:#?}", mint_data);
    println!("\nTransaction Signature: {}", transaction_signature);
    println!("\nAssociated Token Account Address: {}", ata_token_account);
    // println!("Associated Token Account: {:#?}", token_data);
    println!("\nTransaction Signature ATA: {}", transaction_signature_ata);

    // println!("\nToken Account Address: {}", token_account.pubkey());
    // println!("Token Account: {:#?}", token_data_ta);
    // println!("\nTransaction Signature TA: {}", transaction_signature_ta);

    println!("Minted! Tx: {}", sig);
    println!("Token balance: {}", updated_data.amount);

    Ok(())
}
