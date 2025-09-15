use std::net::SocketAddr;

use clap::{Parser, Subcommand};
use linkerd_gateway::{GatewayBuilder, GatewayConfig};
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "linkerd-gateway")]
#[command(about = "CNCF Linkerd2-proxy inspired high-performance gateway for AutoAgents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway server
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to bind to
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,

        /// Upstream service URL
        #[arg(long, default_value = "http://localhost:8081")]
        upstream: String,

        /// Enable authentication
        #[arg(long)]
        auth: bool,

        /// Enable rate limiting
        #[arg(long)]
        rate_limit: bool,

        /// Enable caching
        #[arg(long)]
        cache: bool,

        /// Redis URL for distributed features
        #[arg(long)]
        redis_url: Option<String>,
    },
    /// Show gateway configuration
    Config {
        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "linkerd_gateway=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            host,
            port,
            config,
            upstream,
            auth,
            rate_limit,
            cache,
            redis_url,
        } => {
            let config = load_config(config, upstream, auth, rate_limit, cache, redis_url)?;
            let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

            let gateway = GatewayBuilder::new()
                .with_auth(config.auth)
                .with_rate_limit(config.rate_limit)
                .with_cache(config.cache)
                .with_routing(config.routing)
                .build();

            tracing::info!("Starting Linkerd Gateway on {}", addr);
            tracing::info!("Upstream: {}", upstream);
            tracing::info!("Features: auth={}, rate_limit={}, cache={}",
                auth, rate_limit, cache);

            // Start metrics endpoint
            let metrics_addr: SocketAddr = format!("{}:{}", host, port + 1).parse()?;
            start_metrics_server(metrics_addr, gateway.metrics()).await;

            // Start main gateway server
            tokio::select! {
                result = gateway.serve(addr) => {
                    if let Err(e) = result {
                        tracing::error!("Gateway server error: {}", e);
                        return Err(e);
                    }
                }
                _ = signal::ctrl_c() => {
                    tracing::info!("Received shutdown signal");
                }
            }
        }
        Commands::Config { config } => {
            let config = load_config(config, "http://localhost:8081".to_string(), false, false, false, None)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
    }

    Ok(())
}

/// Load gateway configuration
fn load_config(
    config_path: Option<String>,
    upstream: String,
    auth: bool,
    rate_limit: bool,
    cache: bool,
    redis_url: Option<String>,
) -> Result<GatewayConfig, Box<dyn std::error::Error>> {
    let mut config = if let Some(path) = config_path {
        // Load from file
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)?
    } else {
        // Use default configuration
        GatewayConfig::default()
    };

    // Override with command line arguments
    if auth {
        config.auth.enabled = true;
    }

    if rate_limit {
        config.rate_limit.enabled = true;
        if let Some(url) = redis_url.clone() {
            config.rate_limit.redis_url = Some(url);
        }
    }

    if cache {
        config.cache.enabled = true;
        if let Some(url) = redis_url {
            config.cache.redis_url = Some(url);
        }
    }

    // Set default upstream
    config.routing.default_upstream = Some(upstream);

    Ok(config)
}

/// Start metrics server
async fn start_metrics_server(addr: SocketAddr, metrics: linkerd_gateway::MetricsCollector) {
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Request, Response, Server};
    use std::convert::Infallible;

    let make_svc = make_service_fn(move |_conn| {
        let metrics = metrics.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |_req: Request<Body>| {
                let metrics = metrics.clone();
                async move {
                    let metrics_data = metrics.gather_metrics()
                        .unwrap_or_else(|_| "# Error collecting metrics\n".to_string());

                    Ok::<Response<Body>, Infallible>(
                        Response::builder()
                            .header("content-type", "text/plain; charset=utf-8")
                            .body(Body::from(metrics_data))
                            .unwrap()
                    )
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!("Metrics server error: {}", e);
        }
    });

    tracing::info!("Metrics server listening on {}", addr);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loading() {
        let config = load_config(
            None,
            "http://test:8080".to_string(),
            true,
            true,
            true,
            Some("redis://localhost:6379".to_string()),
        ).unwrap();

        assert!(config.auth.enabled);
        assert!(config.rate_limit.enabled);
        assert!(config.cache.enabled);
        assert_eq!(config.routing.default_upstream, Some("http://test:8080".to_string()));
        assert_eq!(config.rate_limit.redis_url, Some("redis://localhost:6379".to_string()));
    }
}
