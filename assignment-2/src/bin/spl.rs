use anyhow::Result;
use assignment_2::create_key_pair_airdrop_sol;
use mpl_token_metadata::{
    accounts::Metadata,
    instructions::{CreateV1, CreateV1InstructionArgs},
    types::TokenStandard,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    program_pack::Pack,
    signature::{Keypair, Signer, read_keypair_file},
    transaction::Transaction,
};
use solana_system_interface::instruction::create_account;
use spl_associated_token_account_interface::{
    address::get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token_interface::{
    id as token_program_id,
    instruction::{initialize_mint, mint_to, transfer_checked},
    state::Mint,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = RpcClient::new_with_commitment(
        String::from("https://api.devnet.solana.com"),
        CommitmentConfig::confirmed(),
    );

   let payer = read_keypair_file("/Users//.config/solana/phantom.json")
    .expect("Failed to read keypair file");
    println!("Payer: {}", payer.pubkey());

    let mint = Keypair::new();
    println!("Mint: {}", mint.pubkey());

    let recipient = create_key_pair_airdrop_sol(&client).await?;
    println!("Recipient: {}", recipient.pubkey());

    let mint_rent = client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await?;
    let latest_blockhash = client.get_latest_blockhash().await?;

    let payer_ata = get_associated_token_address(&payer.pubkey(), &mint.pubkey());
    let recipient_ata = get_associated_token_address(&recipient.pubkey(), &mint.pubkey());

    let (metadata_pda, _bump) = Metadata::find_pda(&mint.pubkey());
    println!("Metadata PDA: {}", metadata_pda);

    let metadata_ix = CreateV1 {
        metadata: metadata_pda,
        master_edition: None,
        mint: (mint.pubkey(), true),
        authority: payer.pubkey(),
        payer: payer.pubkey(),
        update_authority: (payer.pubkey(), true),
        system_program: solana_sdk::system_program::ID,
        sysvar_instructions: solana_sdk::sysvar::instructions::ID,
        spl_token_program: Some(token_program_id()),
    }
    .instruction(CreateV1InstructionArgs {
        name: String::from("Solana Gold"),
        symbol: String::from("GOLDSOL"),
        uri: String::from("https://raw.githubusercontent.com/solana-developers/program-examples/new-examples/tokens/tokens/.assets/spl-token.json"),
        seller_fee_basis_points: 0,
        primary_sale_happened: false,
        is_mutable: true,
        token_standard: TokenStandard::Fungible,
        decimals: Some(3),
        collection: None,
        uses: None,
        collection_details: None,
        creators: None,
        rule_set: None,
        print_supply: None,
    });

    let tx = Transaction::new_signed_with_payer(
        &[
            create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                mint_rent,
                Mint::LEN as u64,
                &token_program_id(),
            ),
            initialize_mint(
                &token_program_id(),
                &mint.pubkey(),
                &payer.pubkey(),
                Some(&payer.pubkey()),
                3,
            )?,
            metadata_ix,
            create_associated_token_account(
                &payer.pubkey(),
                &payer.pubkey(),
                &mint.pubkey(),
                &token_program_id(),
            ),
            create_associated_token_account(
                &payer.pubkey(),
                &recipient.pubkey(),
                &mint.pubkey(),
                &token_program_id(),
            ),
            mint_to(
                &token_program_id(),
                &mint.pubkey(),
                &payer_ata,
                &payer.pubkey(),
                &[],
                5_000,
            )?,
            transfer_checked(
                &token_program_id(),
                &payer_ata,
                &mint.pubkey(),
                &recipient_ata,
                &payer.pubkey(),
                &[],
                2500,
                3,
            )?,
        ],
        Some(&payer.pubkey()),
        &[&payer, &mint],
        latest_blockhash,
    );

    let sig = client.send_and_confirm_transaction(&tx).await?;
    println!("Transaction: {}", sig);

    Ok(())
}
