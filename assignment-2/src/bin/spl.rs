use anyhow::Result;
use assignment_2;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
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
    instruction::{initialize_mint, mint_to},
    state::Mint,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = RpcClient::new_with_commitment(
        String::from("http://localhost:8899"),
        CommitmentConfig::confirmed(),
    );

    let key_pair = assignment_2::create_key_pair_airdrop_sol(&client).await?;

    let mint = Keypair::new();

    let mint_rent = client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await?;

    let latest_blockhash = client.get_latest_blockhash().await?;
    let recipients = assignment_2::create_key_pair_airdrop_sol(&client).await?;
    let ata_token_account = get_associated_token_address(&key_pair.pubkey(), &mint.pubkey());

    // mint token account - its token account where metadata of token lives

    let setup_transaction = Transaction::new_signed_with_payer(
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
            create_associated_token_account(
                &key_pair.pubkey(),
                &key_pair.pubkey(),
                &mint.pubkey(),
                &token_program_id(),
            ),
            create_associated_token_account(
                &key_pair.pubkey(),
                &recipients.pubkey(),
                &mint.pubkey(),
                &token_program_id(),
            ),
            mint_to(
                &token_program_id(),
                &mint.pubkey(),
                &ata_token_account,
                &key_pair.pubkey(),
                &[],
                1_000_000,
            )?,
        ],
        Some(&key_pair.pubkey()),
        &[&key_pair, &mint],
        latest_blockhash,
    );

    let transaction_signature = client
        .send_and_confirm_transaction(&setup_transaction)
        .await?;

    println!("transaction signature: {}", transaction_signature);

    Ok(())
}
