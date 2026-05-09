use anyhow::Result;
use mpl_core::{
    instructions::CreateV1Builder,
    types::{
        Creator, FreezeDelegate, Plugin, PluginAuthority, PluginAuthorityPair, Royalties,
        RuleSet,
    },
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_client = RpcClient::new("https://api.devnet.solana.com".to_string());

    let keypair = read_keypair_file("/Users//.config/solana/phantom.json").expect("kay pair needed");

    let asset = Keypair::new();

    let create_asset_ix = CreateV1Builder::new()
        .asset(asset.pubkey())
        .payer(keypair.pubkey())
        .name("BlackPearl".into())
        .uri("https://arweave.net/gfO_TkYttQls70pTmhrdMDz9pfMUXX8hZkaoIivQjGs".into())
        .plugins(vec![
            PluginAuthorityPair {
                plugin: Plugin::Royalties(Royalties {
                    basis_points: 500,
                    creators: vec![Creator {
                        address: keypair.pubkey(),
                        percentage: 100,
                    }],
                    rule_set: RuleSet::None,
                }),
                authority: Some(PluginAuthority::UpdateAuthority),
            },
            PluginAuthorityPair {
                plugin: Plugin::FreezeDelegate(FreezeDelegate { frozen: false }),
                authority: Some(PluginAuthority::Owner),
            },
        ])
        .instruction();

    let signers = [&asset, &keypair];

    let latest_blockhash = rpc_client.get_latest_blockhash().await?;

    let create_asset_tx = Transaction::new_signed_with_payer(
        &[create_asset_ix],
        Some(&keypair.pubkey()),
        &signers,
        latest_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&create_asset_tx)
        .await?;

    println!("Asset address: {}", asset.pubkey());
    println!("Signature: {}", signature);

    Ok(())
}
