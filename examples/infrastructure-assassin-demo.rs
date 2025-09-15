//! Infrastructure Assassin Integration Demo
//!
//! This example demonstrates how to integrate Infrastructure Assassin
//! with AutoAgents for unified browser automation and tool orchestration.

use autoagents_core::agent::AgentBuilder;
use infrastructure_assassin::{InfrastructureConfig, InfrastructureAssassin};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Infrastructure Assassin AutoAgents Integration Demo");
    println!("====================================================");

    // Initialize Infrastructure Assassin with default configuration
    println!("✅ Initializing Infrastructure Assassin platform...");
    let config = InfrastructureConfig::default();
    let mut ia = InfrastructureAssassin::init(config).await?;
    println!("   ✓ Platform initialized successfully");

    // Create a sample developer request
    println!("\n✅ Creating sample developer request...");
    let request = infrastructure_assassin::DeveloperRequest {
        description: "Test browser-based web scraping and data extraction".to_string(),
        required_tools: vec![
            "web-scraper".to_string(),
            "data-extractor".to_string(),
            "browser-automation".to_string(),
        ],
        execution_context: {
            let mut ctx = HashMap::new();
            ctx.insert("target_url".to_string(), "https://example.com".to_string());
            ctx.insert("extraction_format".to_string(), "json".to_string());
            ctx
        },
    };

    // Process the request through Infrastructure Assassin
    println!("✅ Processing developer request through Infrastructure Assassin...");
    println!("   Request: {}", request.description);
    println!("   Required tools: {:?}", request.required_tools);

    match ia.process_developer_request(request).await {
        Ok(result) => {
            println!("✅ Request processed successfully!");
            println!("   Session ID: {}", result.session_id);
            println!("   Success: {}", result.success);
            println!("   Output: {}", result.output);
            println!("   Tools Used: {:?}", result.tools_used);
            println!("   Performance: {:.1} MB used, {:.2} CPU cycles",
                    result.memory_used, result.cpu_used);

            // Show analytics data
            println!("\n📊 Infrastructure Assassin Analytics:");
            let analytics = ia.analytics_tracker.get_revenue_analytics();
            println!("   Total tool orchestrations: {}", analytics.tool_orchestrations);
            println!("   Productivity gain: ${:.2}", analytics.productivity_gain);
            println!("   AWS cost saved: ${:.2}", analytics.aws_cost_saved);
        }
        Err(e) => {
            println!("❌ Failed to process request: {}", e);
            return Err(e.into());
        }
    }

    // Demonstrate browser spawning capability
    println!("\n🌐 Testing browser spawning capability...");
    let browser_config = infrastructure_assassin::browser::BrowserConfig::default();

    println!("   Browser config: {}x{}, headless={}, sandboxed={}",
             browser_config.width, browser_config.height,
             browser_config.headless, browser_config.sandboxed);

    // Attempt to spawn browser (will work in WASM environment)
    match ia.browser_factory.spawn_ephemeral_browser(browser_config).await {
        Ok(session) => {
            println!("✅ Browser session spawned successfully!");
            println!("   Session ID: {}", session.session_id);

            // Destroy the session
            match ia.browser_factory.destroy_session(session).await {
                Ok(()) => println!("✅ Browser session destroyed successfully"),
                Err(e) => println!("❌ Failed to destroy session: {}", e),
            }
        }
        Err(e) => {
            println!("ℹ️  Browser spawning unavailable in current environment: {}", e);
            println!("   (This is expected in non-WASM environments)");
        }
    }

    // Show competitive analysis
    println!("\n💰 Cost Disruption Analysis:");
    let analysis = ia.analytics_tracker.generate_competitive_analysis();
    println!("   AWS Serverless Cost: ${:.1}/month", analysis.aws_serverless_cost);
    println!("   Google Serverless Cost: ${:.1}/month", analysis.google_serverless_cost);
    println!("   Infrastructure Assassin Cost: ${:.2}/month",
             analysis.infrastructure_assassin_cost);
    println!("   Productivity Multiplier: {:.0}x", analysis.productivity_multiplier);
    println!("   Tool Ecosystem Size: {}+", analysis.tool_ecosystem_size);

    let disruption_ratio = (analysis.aws_serverless_cost +
                           analysis.google_serverless_cost) /
                           (24.0 * 30.0); // Convert to hourly equivalent
    println!("   Potential savings vs AWS/Google: ${:.4}/hour", disruption_ratio);

    println!("\n🎉 Infrastructure Assassin Demo Complete!");
    println!("   Infrastructure Assassin is ready for integration with AutoAgents!");
    println!("   Next steps:");
    println!("   1. Deploy as WASM module in web environment");
    println!("   2. Integrate MCP servers for additional tool capabilities");
    println!("   3. Add enterprise metrics and monitoring");
    println!("   4. Enable revenue generation through productivity gains");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infrastructure_assassin_integration() {
        // This test would run in WASM environment
        println!("Infrastructure Assassin integration test placeholder");
        assert!(true);
    }
}
