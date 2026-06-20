use pezkuwi_subxt::{backend::rpc::RpcClient, OnlineClient};

#[async_trait::async_trait]
pub trait ClientFromUrl: Sized {
	async fn from_secure_url(url: &str) -> Result<Self, pezkuwi_subxt::Error>;
	async fn from_insecure_url(url: &str) -> Result<Self, pezkuwi_subxt::Error>;
}

#[async_trait::async_trait]
impl<Config: pezkuwi_subxt::Config + Send + Sync> ClientFromUrl for OnlineClient<Config> {
	async fn from_secure_url(url: &str) -> Result<Self, pezkuwi_subxt::Error> {
		Self::from_url(url).await.map_err(Into::into)
	}

	async fn from_insecure_url(url: &str) -> Result<Self, pezkuwi_subxt::Error> {
		Self::from_insecure_url(url).await.map_err(Into::into)
	}
}

#[async_trait::async_trait]
impl ClientFromUrl for RpcClient {
	async fn from_secure_url(url: &str) -> Result<Self, pezkuwi_subxt::Error> {
		Self::from_url(url).await.map_err(pezkuwi_subxt::Error::from)
	}

	async fn from_insecure_url(url: &str) -> Result<Self, pezkuwi_subxt::Error> {
		Self::from_insecure_url(url).await.map_err(pezkuwi_subxt::Error::from)
	}
}

pub async fn get_client_from_url<T: ClientFromUrl + Send>(
	url: &str,
) -> Result<T, pezkuwi_subxt::Error> {
	if pezkuwi_subxt::utils::url_is_secure(url)? {
		T::from_secure_url(url).await
	} else {
		T::from_insecure_url(url).await
	}
}
