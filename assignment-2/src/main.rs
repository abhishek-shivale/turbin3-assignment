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
use spl_token_interface::{id as token_program_id, instruction::initialize_mint, state::Mint};

#[tokio::main]
async fn main() -> Result<()> {
    let client = RpcClient::new_with_commitment(
        String::from("http://localhost:8899"),
        CommitmentConfig::confirmed(),
    );

    let key_pair = Keypair::new();

    let mint = Keypair::new();

    let lamports = LAMPORTS_PER_SOL * 10;

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

    println!("Mint Address: {}", mint.pubkey());
    println!("Mint Account: {:#?}", mint_data);
    println!("\nTransaction Signature: {}", transaction_signature);

    Ok(())
}
