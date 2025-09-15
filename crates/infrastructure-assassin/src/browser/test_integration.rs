//! Integration tests and examples for enhanced browser APIs
//!
//! This module provides integration tests and usage examples for the
//! enhanced browser automation capabilities.

/// Example usage of enhanced browser automation APIs
pub async fn browser_automation_demo() -> Result<(), crate::Error> {
    log::info!("Starting Infrastructure Assassin browser automation demo");

    // 1. Initialize browser session
    let config = crate::browser::BrowserConfig::default();
    let _session = crate::browser::spawn_ephemeral_browser(config)?;

    // 2. DOM manipulation example
    use crate::browser::*;
    let selector = ".content";
    let action = DomAction::SetText { content: "Infrastructure Assassin Active".to_string() };
    manipulate_dom(selector, action).await?;

    // 3. JavaScript execution example
    let js_code = r#"
        console.log('Infrastructure Assassin JavaScript execution');
        return { status: 'active', timestamp: Date.now() };
    "#;
    let result = execute_script(js_code).await?;
    log::info!("JavaScript execution result: {:?}", result);

    // 4. Inject browser utilities
    inject_browser_utilities().await?;

    // 5. Monitor console output
    monitor_console_output().await?;

    // 6. Network monitoring example
    let mut network_events = intercept_fetch("api.example.com").await?;
    log::info!("Network interception active");

    // 7. Storage operations example
    let session_state = SessionState {
        session_id: "demo-session".to_string(),
        agent_states: std::collections::HashMap::new(),
        user_context: UserContext {
            user_id: None,
            preferences: std::collections::HashMap::new(),
            device_info: DeviceInfo {
                user_agent: "Demo/1.0".to_string(),
                viewport_width: 1920,
                viewport_height: 1080,
                pixel_ratio: 1.0,
                language: "en".to_string(),
                timezone: "UTC".to_string(),
            },
        },
        timestamp: js_sys::Date::now(),
        version: "2.0.0".to_string(),
    };
    store_session_state("demo", session_state).await?;
    log::info!("Session state stored");

    // 8. Screenshot capabilities
    let screenshot_data = capture_viewport().await?;
    log::info!("Viewport screenshot captured: {} bytes", screenshot_data.len());

    // 9. Generate dashboard
    let dashboard_data = render_dashboard().await?;
    log::info!("Dashboard rendered: {} bytes", dashboard_data.len());

    log::info!("✅ Infrastructure Assassin browser automation demo completed successfully");
    Ok(())
}

/// Example of creating a real browser session with full capabilities
pub async fn create_enhanced_browser_session() -> Result<RealBrowserSession, crate::Error> {
    let config = crate::browser::BrowserConfig {
        headless: false, // Run with visual browser for demo
        width: 1920,
        height: 1080,
        timeout_ms: 60000,
        user_agent: Some("Infrastructure-Assassin-Demo/2.0".to_string()),
        sandboxed: true,
        enable_mcp_integration: true,
    };

    let session = create_real_browser_session(config);
    log::info!("Created enhanced browser session: {}", session.session_id);
    session
}

/// Example workflow: Automated form filling and submission
pub async fn automated_form_workflow(form_selector: &str, data: std::collections::HashMap<&str, &str>) -> Result<(), crate::Error> {
    // Fill form fields
    for (field_name, value) in data {
        let selector = format!("{} [name='{}']", form_selector, field_name);
        let action = DomAction::Type { text: value.to_string() };
        manipulate_dom(&selector, action).await?;
        log::debug!("Filled field '{}': {}", field_name, value);
    }

    // Submit form
    let submit_selector = format!("{} [type='submit']", form_selector);
    let submit_action = DomAction::Click;
    manipulate_dom(&submit_selector, submit_action).await?;
    log::info!("Form submitted automatically");

    Ok(())
}

/// Example workflow: Content extraction and analysis
pub async fn content_extraction_workflow() -> Result<String, crate::Error> {
    // Extract complete page content
    let content = extract_page_content().await?;

    // Execute custom analysis script
    let analysis_script = r#"
        (function() {
            const links = Array.from(document.querySelectorAll('a[href]'));
            const images = Array.from(document.querySelectorAll('img[src]'));
            const forms = Array.from(document.querySelectorAll('form'));

            return {
                linkCount: links.length,
                imageCount: images.length,
                formCount: forms.length,
                externalLinks: links.filter(a => a.href.startsWith('http') && !a.href.includes(window.location.hostname)).length,
                analysisTimestamp: Date.now()
            };
        })()
    "#;

    let analysis_result = evaluate_expression(analysis_script).await?;
    log::info!("Content analysis completed: {:?}", analysis_result);

    Ok(content)
}

/// Example workflow: Network monitoring during page navigation
pub async fn network_monitoring_workflow(urls: Vec<String>) -> Result<NetworkMetrics, crate::Error> {
    // Start network monitoring
    let mut network_stream = intercept_fetch("*").await?;

    // Perform network operations
    let latencies = measure_latencies(urls).await?;
    log::info!("Network latency measurements: {:?}", latencies);

    // Get analytics
    let metrics = get_network_analytics().await?;
    log::info!("Network analytics: {} requests, {} bytes", metrics.total_requests, metrics.total_response_size);

    Ok(metrics)
}

/// Example workflow: Visual proof generation
pub async fn visual_proof_workflow() -> Result<(Vec<u8>, Vec<u8>), crate::Error> {
    // Create visual agent UI
    let agent_config = AgentConfig {
        agent_type: "Demo Agent".to_string(),
        capabilities: vec!["DOM Manipulation".to_string(), "JS Execution".to_string()],
        selectors: std::collections::HashMap::new(),
        event_handlers: vec!["click".to_string(), "submit".to_string()],
    };

    inject_agent_ui(".container", agent_config).await?;

    // Capture before/after screenshots
    let before_screenshot = capture_viewport().await?;
    log::info!("Before screenshot: {} bytes", before_screenshot.len());

    // Make changes to the page
    let action = DomAction::SetText { content: "Infrastructure Assassin Demo Active".to_string() };
    manipulate_dom("h1", action).await?;

    let after_screenshot = capture_viewport().await?;
    log::info!("After screenshot: {} bytes", after_screenshot.len());

    Ok((before_screenshot, after_screenshot))
}

/// Validate that all browser APIs are working correctly
pub async fn validate_enhanced_apis() -> Result<bool, crate::Error> {
    let mut validations = Vec::new();

    // 1. Test DOM manipulation
    validations.push(("DOM Manipulation", manipulate_dom("body", DomAction::SetAttribute {
        name: "data-ia-validated".to_string(),
        value: "true".to_string()
    }).await.is_ok()));

    // 2. Test JavaScript execution
    validations.push(("JavaScript Execution", execute_script("return 'IA validation';").await.is_ok()));

    // 3. Test storage operations
    let test_state = SessionState {
        session_id: "validation-test".to_string(),
        agent_states: std::collections::HashMap::new(),
        user_context: UserContext {
            user_id: None,
            preferences: std::collections::HashMap::new(),
            device_info: DeviceInfo {
                user_agent: "Validation/1.0".to_string(),
                viewport_width: 800,
                viewport_height: 600,
                pixel_ratio: 1.0,
                language: "en".to_string(),
                timezone: "UTC".to_string(),
            },
        },
        timestamp: js_sys::Date::now(),
        version: "validation".to_string(),
    };
    validations.push(("Storage Operations", store_session_state("validation", test_state).await.is_ok()));

    // 4. Test screenshot capture
    validations.push(("Screenshot Capture", capture_viewport().await.is_ok()));

    // 5. Test network monitoring
    validations.push(("Network Monitoring", measure_latencies(vec!["https://httpbin.org/get".to_string()]).await.is_ok()));

    let passed = validations.iter().filter(|(_, pass)| *pass).count();
    let total = validations.len();

    log::info!("Browser API validation: {}/{} tests passed", passed, total);
    for (name, passed_test) in validations {
        log::info!("  {}: {}", name, if passed_test { "✅ PASSED" } else { "❌ FAILED" });
    }

    Ok(passed == total)
}

// Required imports for the integration test
use crate::browser::{
    enhanced::*,
    js_execution::*,
    network::*,
    storage::*,
    screenshot::*,
};
use crate::browser::BrowserConfig;
