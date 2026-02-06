use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub service_type: String, // Should be "subscription_tier1", "subscription_tier2", or "proof_generation"
    pub user_address: String,
    pub duration_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub quote: serde_json::Value,
    pub signature: serde_json::Value, // Signature is a struct, not a string
    pub hash: String,
}

pub struct ProxyClient {
    client: Client,
    proxy_url: String,
}

impl ProxyClient {
    pub fn new(proxy_url: &str) -> Self {
        Self {
            client: Client::new(),
            proxy_url: proxy_url.to_string(),
        }
    }

    /// Request a quote for a specific service tier.
    pub async fn request_quote(&self, req: QuoteRequest) -> Result<QuoteResponse, Box<dyn std::error::Error>> {
        let resp = self.client
            .post(format!("{}/api/v1/quote", self.proxy_url))
            .json(&req)
            .send()
            .await?;
            
        let status = resp.status();
        let text = resp.text().await?;
        
        if !status.is_success() {
            return Err(format!("Proxy returned error status {}: {}", status, text).into());
        }
        
        match serde_json::from_str::<QuoteResponse>(&text) {
            Ok(quote_resp) => Ok(quote_resp),
            Err(e) => Err(e.into())
        }
    }

    /// Perform a proxied request with the required user address header.
    pub async fn proxy_get(&self, path: &str, user_address: &str) -> Result<Response, reqwest::Error> {
        self.client
            .get(format!("{}{}", self.proxy_url, path))
            .header("X-User-Address", user_address)
            .send()
            .await
    }
    
    // Additional methods for POST, PUT etc can be added similarly
}
