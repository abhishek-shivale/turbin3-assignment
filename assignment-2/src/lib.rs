use anyhow::{Ok, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    signature::{Keypair, Signer},
};

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

pub async fn create_key_pair_airdrop_sol(client: &RpcClient) -> Result<Keypair> {
    let key_pair = Keypair::new();

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

    Ok(key_pair)
}
