use crate::helpers::configuration::CastConfig;
use crate::{get_provider, Network};
use anyhow::{bail, Context, Result};
use clap::Args;
use shared::verify_and_warn_if_incompatible_rpc_version;
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
use std::env::current_exe;
use std::time::UNIX_EPOCH;

#[derive(Args, Clone, Debug, Default)]
#[group(required = false, multiple = false)]
pub struct RpcArgs {
    /// RPC provider url address; overrides url from snfoundry.toml
    #[clap(short, long)]
    pub url: Option<String>,

    /// Use predefined network using public provider
    #[clap(long)]
    pub network: Option<Network>,
}

impl RpcArgs {
    pub async fn get_provider(&self, config: &CastConfig) -> Result<JsonRpcClient<HttpTransport>> {
        if self.network.is_some() && !config.url.is_empty() {
            bail!("The argument '--network' cannot be used when `url` is defined in `snfoundry.toml` for the active profile")
        }

        let url = if let Some(network) = self.network {
            let free_provider = FreeProvider::semi_random();
            network.url(&free_provider)
        } else {
            let url = self.url.clone().or_else(|| {
                if config.url.is_empty() {
                    None
                } else {
                    Some(config.url.clone())
                }
            });

            url.context("Either `--network` or `--url` must be provided")?
        };

        assert!(!url.is_empty(), "url cannot be empty");
        let provider = get_provider(&url)?;

        verify_and_warn_if_incompatible_rpc_version(&provider, url).await?;

        Ok(provider)
    }

    #[must_use]
    pub fn get_url(&self, config: &CastConfig) -> String {
        self.url.clone().unwrap_or_else(|| config.url.clone())
    }
}

fn installation_constant_seed() -> Result<u64> {
    let executable_path = current_exe()?;
    let metadata = executable_path.metadata()?;
    let modified_time = metadata.modified()?;
    let duration = modified_time.duration_since(UNIX_EPOCH)?;

    Ok(duration.as_secs())
}

enum FreeProvider {
    Blast,
    Voyager,
}

impl FreeProvider {
    fn semi_random() -> Self {
        let seed = installation_constant_seed().unwrap_or(2);
        if seed % 2 == 0 {
            return Self::Blast;
        }
        Self::Voyager
    }
}

impl Network {
    fn url(self, provider: &FreeProvider) -> String {
        match self {
            Network::Mainnet => Self::free_mainnet_rpc(provider),
            Network::Sepolia => Self::free_sepolia_rpc(provider),
        }
    }

    fn free_mainnet_rpc(provider: &FreeProvider) -> String {
        match provider {
            FreeProvider::Blast => {
                "https://starknet-mainnet.public.blastapi.io/rpc/v0_7".to_string()
            }
            FreeProvider::Voyager => "https://free-rpc.nethermind.io/mainnet-juno".to_string(),
        }
    }

    fn free_sepolia_rpc(provider: &FreeProvider) -> String {
        match provider {
            FreeProvider::Blast => {
                "https://starknet-sepolia.public.blastapi.io/rpc/v0_7".to_string()
            }
            FreeProvider::Voyager => "https://free-rpc.nethermind.io/sepolia-juno".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Response;
    use test_case::test_case;

    async fn call_provider(url: &str) -> Result<Response> {
        let client = reqwest::Client::new();
        client
            .get(url)
            .send()
            .await
            .context("Failed to send request")
    }

    #[test_case(FreeProvider::Voyager)]
    #[test_case(FreeProvider::Blast)]
    #[tokio::test]
    async fn test_mainnet_url_works(free_provider: FreeProvider) {
        assert!(call_provider(&Network::free_mainnet_rpc(&free_provider))
            .await
            .is_ok());
    }

    #[test_case(FreeProvider::Voyager)]
    #[test_case(FreeProvider::Blast)]
    #[tokio::test]
    async fn test_sepolia_url_works(free_provider: FreeProvider) {
        assert!(call_provider(&Network::free_sepolia_rpc(&free_provider))
            .await
            .is_ok());
    }
}
