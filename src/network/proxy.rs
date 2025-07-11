//manage 50 proxies
// integrate with ActivitySimulator and mixer/connectors 

// Action: Use reqwest::Proxy with a pool of proxy URLs.
// Metric: Rotate proxies for 10 API calls and 10 blockchain transactions.

// src/network/proxy.rs
use crate::error::WalletError;
use reqwest::{Client, Proxy};
use rand::seq::SliceRandom;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::info;

/// Manages a pool of proxy URLs and rotates them for API and blockchain requests.
#[derive(Clone)]
pub struct ProxyManager {
    proxies: Arc<Mutex<Vec<String>>>, // Thread-safe proxy pool
    client_cache: Arc<Mutex<Vec<Client>>>, // Cache of pre-configured clients
}

impl ProxyManager {
    /// Initialize ProxyManager with a list of proxy URLs.
    pub fn new(proxies: Vec<String>) -> Result<Self, WalletError> {
        if proxies.is_empty() {
            return Err(WalletError::MixingError("No proxies provided".to_string()));
        }
        if proxies.len() < 50 {
            log::warn!("Fewer than 50 proxies provided ({}). Consider adding more for better rotation.", proxies.len());
        }
        Ok(Self {
            proxies: Arc::new(Mutex::new(proxies)),
            client_cache: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Get a reqwest::Client configured with a randomly selected proxy.
    pub async fn get_client(&self) -> Result<Client, WalletError> {
        let proxies = self.proxies.lock().await;
        let proxy_url = proxies
            .choose(&mut rand::thread_rng())
            .ok_or_else(|| WalletError::MixingError("No proxies available".to_string()))?;

        // Check cache for existing client with this proxy
        let mut cache = self.client_cache.lock().await;
        if let Some(client) = cache.iter().find(|c| {
            c.proxy().map(|p| p.url().to_string()).unwrap_or_default() == *proxy_url
        }) {
            info!("Reusing cached client for proxy: {}", proxy_url);
            return Ok(client.clone());
        }

        // Create new client with proxy
        let proxy = Proxy::all(proxy_url)
            .map_err(|e| WalletError::MixingError(format!("Failed to create proxy: {}", e)))?;
        let client = Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| WalletError::MixingError(format!("Failed to build client: {}", e)))?;

        cache.push(client.clone());
        info!("Created new client for proxy: {}", proxy_url);
        Ok(client)
    }

    /// Add a new proxy to the pool.
    pub async fn add_proxy(&self, proxy_url: String) -> Result<(), WalletError> {
        let mut proxies = self.proxies.lock().await;
        if !proxies.contains(&proxy_url) {
            proxies.push(proxy_url.clone());
            info!("Added proxy: {}", proxy_url);
        }
        Ok(())
    }

    /// Remove a proxy from the pool.
    pub async fn remove_proxy(&self, proxy_url: &str) -> Result<(), WalletError> {
        let mut proxies = self.proxies.lock().await;
        if let Some(index) = proxies.iter().position(|p| p == proxy_url) {
            proxies.remove(index);
            info!("Removed proxy: {}", proxy_url);
            // Clear cache to force refresh of clients
            self.client_cache.lock().await.clear();
        }
        Ok(())
    }

    /// Get the current number of proxies.
    pub async fn proxy_count(&self) -> usize {
        self.proxies.lock().await.len()
    }
}