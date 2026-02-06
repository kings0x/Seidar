use sdk_rust::client::{ProxyClient, QuoteRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ProxyClient::new("http://localhost:8080");

    // 1. Request a Quote
    let quote_req = QuoteRequest {
        service_type: "subscription_tier1".to_string(),
        user_address: "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266".to_string(),
        duration_seconds: Some(3600),
    };

    println!("Requesting quote for: {}", quote_req.service_type);
    match client.request_quote(quote_req).await {
        Ok(resp) => println!("Quote received: {:?}", resp.hash),
        Err(e) => eprintln!("Error requesting quote: {}", e),
    }

    // 2. Perform a proxied request
    println!("Performing proxied request...");
    let res = client.proxy_get("/", "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266").await?;
    println!("Response Status: {}", res.status());
    let body = res.text().await?;
    println!("Response Body: {}", body);

    Ok(())
}
