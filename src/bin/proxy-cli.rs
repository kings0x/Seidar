use clap::{Parser, Subcommand};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;

#[derive(Parser)]
#[command(name = "proxy-cli")]
#[command(about = "Management CLI for the Rust Reverse Proxy", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "http://localhost:8081")]
    url: String,

    #[arg(short, long, default_value = "admin-secret-key")]
    key: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check proxy system status
    Status,
    /// List backend health and connections
    Backends,
    /// View real-time analytics
    Analytics,
    /// Inspect subscription cache
    Cache,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", cli.key))?,
    );

    match cli.command {
        Commands::Status => {
            let res = client.get(format!("{}/admin/status", cli.url))
                .headers(headers)
                .send()
                .await?;
            print_response(res).await?;
        }
        Commands::Backends => {
            let res = client.get(format!("{}/admin/backends", cli.url))
                .headers(headers)
                .send()
                .await?;
            print_response(res).await?;
        }
        Commands::Analytics => {
            let res = client.get(format!("{}/admin/analytics", cli.url))
                .headers(headers)
                .send()
                .await?;
            print_response(res).await?;
        }
        Commands::Cache => {
            let res = client.get(format!("{}/admin/cache", cli.url))
                .headers(headers)
                .send()
                .await?;
            print_response(res).await?;
        }
    }

    Ok(())
}

async fn print_response(res: reqwest::Response) -> Result<(), Box<dyn std::error::Error>> {
    let status = res.status();
    if !status.is_success() {
        eprintln!("Error: Admin API returned status {}", status);
        if let Ok(text) = res.text().await {
            eprintln!("Response: {}", text);
        }
        return Ok(());
    }

    let json: Value = res.json().await?;
    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}
