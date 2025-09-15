//! Network monitoring and interception capabilities
//!
//! This module provides comprehensive network request monitoring, interception,
//! mocking, and analytics for browser automation and performance tracking.

use crate::Error;
use futures::Stream;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window, Request, RequestInit, Response, FetchEvent, ServiceWorkerGlobalScope};
use js_sys::{Array, Object, Promise, Reflect};
use std::collections::HashMap;

/// Network event types for monitoring
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Request { url: String, method: String, headers: HashMap<String, String>, timestamp: f64 },
    Response { url: String, status: u16, status_text: String, content_type: Option<String>, size: usize, duration: f64 },
    Error { url: String, error: String, timestamp: f64 },
    Intercepted { url: String, original_request: Box<NetworkEvent>, modified_request: Option<Box<NetworkEvent>> },
}

/// Network request interception configuration
#[derive(Debug, Clone)]
pub struct NetworkInterceptorConfig {
    pub url_pattern: String,
    pub methods: Vec<String>,
    pub headers_to_modify: HashMap<String, String>,
    pub response_to_mock: Option<MockResponse>,
    pub delay_ms: Option<u64>,
}

/// Mock response for testing
#[derive(Debug, Clone)]
pub struct MockResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub delay_ms: u64,
}

/// Network performance metrics
#[derive(Debug, Clone)]
pub struct NetworkMetrics {
    pub total_requests: u64,
    pub total_response_size: u64,
    pub average_latency: f64,
    pub slowest_request: Option<(String, f64)>,
    pub request_types: HashMap<String, u64>,
}

/// Intercept fetch requests with custom patterns
pub async fn intercept_fetch(request_pattern: &str) -> Result<impl Stream<Item = NetworkEvent>, Error> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    // Check if Service Worker is available for interception
    if let Ok(service_worker) = js_sys::global().dyn_into::<ServiceWorkerGlobalScope>() {
        // Use Service Worker for network interception
        intercept_with_service_worker(&service_worker, request_pattern, tx.clone())?;
    } else {
        // Use fetch override for network monitoring
        intercept_with_fetch_override(request_pattern, tx.clone())?;
    }

    // Also intercept XMLHttpRequest
    intercept_xmlhttprequest(request_pattern, tx.clone())?;

    log::info!("Network interception active for pattern: {}", request_pattern);
    Ok(rx)
}

/// Intercept using Service Worker (when available)
fn intercept_with_service_worker(
    service_worker: &ServiceWorkerGlobalScope,
    pattern: &str,
    tx: futures::channel::mpsc::UnboundedSender<NetworkEvent>,
) -> Result<(), Error> {
    let pattern_clone = pattern.to_string();

    let fetch_handler = Closure::wrap(Box::new(move |event: JsValue| {
        if let Ok(fetch_event) = event.dyn_into::<FetchEvent>() {
            let url = fetch_event.request().url();
            let method = fetch_event.request().method();

            // Check if URL matches pattern
            if url.contains(&pattern_clone) {
                let timestamp = js_sys::Date::now();

                let network_event = NetworkEvent::Request {
                    url: url.clone(),
                    method: method.clone(),
                    headers: HashMap::new(), // TODO: Extract headers
                    timestamp,
                };

                let _ = tx.unbounded_send(network_event);

                // Log interception
                console::log_1(&JsValue::from_str(&format!(
                    "Infrastructure Assassin intercepted: {} {}", method, url
                )));
            }
        }
    }) as Box<dyn FnMut(JsValue)>);

    if let Ok(add_event_listener) = Reflect::get(service_worker, &JsValue::from_str("addEventListener")) {
        if let Ok(function) = add_event_listener.dyn_into::<js_sys::Function>() {
            function.call2(
                service_worker.as_ref(),
                &JsValue::from_str("fetch"),
                fetch_handler.as_ref().unchecked_ref(),
            ).ok();
        }
    }

    fetch_handler.forget();
    Ok(())
}

/// Intercept using fetch override (fallback method)
fn intercept_with_fetch_override(
    pattern: &str,
    tx: futures::channel::mpsc::UnboundedSender<NetworkEvent>,
) -> Result<(), Error> {
    let original_fetch = Reflect::get(&js_sys::global(), &JsValue::from_str("fetch"))
        .map_err(|_| Error::BrowserAutomation("Cannot access original fetch".to_string()))?;

    let pattern_clone = pattern.to_string();

    let fetch_override = Closure::wrap(Box::new(move |args: &JsValue| -> Promise {
        let array = js_sys::Array::from(args);

        if array.length() >= 1 {
            if let Ok(request_like) = array.get(0) {
                let url = if let Ok(request) = request_like.dyn_into::<Request>() {
                    request.url()
                } else if let Ok(url_str) = request_like.as_string() {
                    url_str
                } else {
                    String::new()
                };

                if url.contains(&pattern_clone) {
                    let method = if array.length() >= 2 {
                        if let Ok(options) = array.get(1).dyn_into::<RequestInit>() {
                            options.method().unwrap_or_else(|| "GET".to_string())
                        } else {
                            "GET".to_string()
                        }
                    } else {
                        "GET".to_string()
                    };

                    let timestamp = js_sys::Date::now();

                    let network_event = NetworkEvent::Request {
                        url: url.clone(),
                        method: method.clone(),
                        headers: HashMap::new(),
                        timestamp,
                    };

                    let _ = tx.unbounded_send(network_event);
                }
            }
        }

        // Call original fetch
        if let Ok(function) = original_fetch.dyn_into::<js_sys::Function>() {
            function.apply(&js_sys::global(), &array)
                .unwrap_or_else(|_| Promise::resolve(&JsValue::NULL))
        } else {
            Promise::resolve(&JsValue::NULL)
        }
    }) as Box<dyn FnMut(&JsValue) -> Promise>);

    Reflect::set(&js_sys::global(), &JsValue::from_str("fetch"), fetch_override.as_ref().unchecked_ref())
        .map_err(|_| Error::BrowserAutomation("Failed to override fetch".to_string()))?;

    fetch_override.forget();
    Ok(())
}

/// Intercept XMLHttpRequest calls
fn intercept_xmlhttprequest(
    pattern: &str,
    tx: futures::channel::mpsc::UnboundedSender<NetworkEvent>,
) -> Result<(), Error> {
    let xhr_override = format!(r#"
        (function() {{
            const originalOpen = XMLHttpRequest.prototype.open;
            const originalSend = XMLHttpRequest.prototype.send;
            const pattern = "{}";

            XMLHttpRequest.prototype.open = function(method, url, async, user, password) {{
                this._ia_url = url;
                this._ia_method = method;
                this._ia_timestamp = Date.now();

                // Check if URL matches pattern
                if (url && url.includes(pattern)) {{
                    window.infrastructureAssassinNetworkEvent({{
                        type: 'request',
                        url: url,
                        method: method,
                        timestamp: this._ia_timestamp
                    }});
                }}

                return originalOpen.call(this, method, url, async !== false, user, password);
            }};

            XMLHttpRequest.prototype.send = function(body) {{
                if (this._ia_url && this._ia_url.includes(pattern)) {{
                    this.addEventListener('load', function() {{
                        window.infrastructureAssassinNetworkEvent({{
                            type: 'response',
                            url: this._ia_url,
                            status: this.status,
                            statusText: this.statusText,
                            contentType: this.getResponseHeader('content-type'),
                            size: this.responseText ? this.responseText.length : 0,
                            duration: Date.now() - this._ia_timestamp
                        }});
                    }});

                    this.addEventListener('error', function() {{
                        window.infrastructureAssassinNetworkEvent({{
                            type: 'error',
                            url: this._ia_url,
                            error: 'XMLHttpRequest failed',
                            timestamp: Date.now()
                        }});
                    }});
                }}

                return originalSend.call(this, body);
            }};
        }})()
    "#, pattern);

    js_sys::eval(&xhr_override)
        .map_err(|_| Error::BrowserAutomation("Failed to override XMLHttpRequest".to_string()))?;

    Ok(())
}

/// Mock network responses for testing
pub async fn mock_responses() -> Result<(), Error> {
    let mock_setup = r#"
        (function() {
            // Create mock response utilities
            window.infrastructureAssassinNetworkMock = {
                mocks: new Map(),
                requestCount: 0,

                addMock: function(url, mockResponse) {
                    this.mocks.set(url, mockResponse);
                    console.log('Infrastructure Assassin mock added for:', url);
                },

                getMock: function(url) {
                    return this.mocks.get(url);
                },

                clearMocks: function() {
                    this.mocks.clear();
                    console.log('Infrastructure Assassin mocks cleared');
                }
            };

            // Global event handler for network events
            window.infrastructureAssassinNetworkEvent = function(event) {
                if (window.infrastructureAssassinNetworkCallback) {
                    window.infrastructureAssassinNetworkCallback(event);
                }
            };

            console.log('Infrastructure Assassin network mocking active');
        })()
    "#;

    js_sys::eval(mock_setup)
        .map_err(|_| Error::BrowserAutomation("Failed to initialize network mocking".to_string()))?;

    log::info!("Network response mocking activated");
    Ok(())
}

/// Measure network latencies
pub async fn measure_latencies(urls: Vec<String>) -> Result<HashMap<String, f64>, Error> {
    let mut latencies = HashMap::new();

    for url in urls {
        let start_time = js_sys::Date::now();

        let test_request = format!(r#"
            (function() {{
                return fetch('{}', {{
                    method: 'HEAD',
                    cache: 'no-cache'
                }})
                .then(response => ({{
                    status: response.status,
                    duration: Date.now() - {}
                }}))
                .catch(error => ({{
                    error: error.message,
                    duration: Date.now() - {}
                }}));
            }})()
        "#, url, start_time, start_time);

        if let Ok(result) = js_sys::eval(&test_request) {
            if let Ok(duration) = Reflect::get(&result, &JsValue::from_str("duration")) {
                if let Ok(latency) = duration.as_f64() {
                    latencies.insert(url, latency);
                    log::debug!("Measured latency for {}: {}ms", url, latency);
                }
            }
        }
    }

    log::info!("Measured network latencies for {} URLs", latencies.len());
    Ok(latencies)
}

/// Get network analytics and metrics
pub async fn get_network_analytics() -> Result<NetworkMetrics, Error> {
    let analytics_script = r#"
        (function() {
            const resources = performance.getEntriesByType('resource');
            const navigation = performance.getEntriesByType('navigation')[0];

            const requestTypes = {};
            let totalSize = 0;
            let totalRequests = resources.length;

            resources.forEach(resource => {
                const type = resource.initiatorType || 'other';
                requestTypes[type] = (requestTypes[type] || 0) + 1;

                // Estimate size if transferSize is available
                if (resource.transferSize) {
                    totalSize += resource.transferSize;
                }
            });

            return {
                totalRequests: totalRequests,
                totalResponseSize: totalSize,
                requestTypes: requestTypes,
                navigationTiming: {
                    domContentLoaded: navigation ? navigation.domContentLoadedEventEnd - navigation.domContentLoadedEventStart : 0,
                    loadComplete: navigation ? navigation.loadEventEnd - navigation.loadEventStart : 0
                }
            };
        })()
    "#;

    let result = js_sys::eval(analytics_script)
        .map_err(|_| Error::BrowserAutomation("Failed to get network analytics".to_string()))?;

    let total_requests = Reflect::get(&result, &JsValue::from_str("totalRequests"))
        .unwrap_or(JsValue::from_f64(0.0))
        .as_f64()
        .unwrap_or(0.0) as u64;

    let total_size = Reflect::get(&result, &JsValue::from_str("totalResponseSize"))
        .unwrap_or(JsValue::from_f64(0.0))
        .as_f64()
        .unwrap_or(0.0) as u64;

    let request_types = Reflect::get(&result, &JsValue::from_str("requestTypes"))
        .unwrap_or(JsValue::NULL);

    let mut request_types_map = HashMap::new();
    if request_types.is_object() {
        let keys = Reflect::own_keys(&request_types).unwrap_or(Array::new());
        for i in 0..keys.length() {
            if let Ok(key) = keys.get(i).as_string() {
                if let Ok(value) = Reflect::get(&request_types, &keys.get(i)) {
                    if let Ok(count) = value.as_f64() {
                        request_types_map.insert(key, count as u64);
                    }
                }
            }
        }
    }

    let metrics = NetworkMetrics {
        total_requests,
        total_response_size: total_size,
        average_latency: 0.0, // TODO: Calculate from individual requests
        slowest_request: None,
        request_types: request_types_map,
    };

    log::info!("Collected network analytics: {} requests, {} bytes", total_requests, total_size);
    Ok(metrics)
}

/// Capture all network requests during a time period
pub async fn capture_network_requests(duration_ms: u64) -> Result<Vec<NetworkEvent>, Error> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    // Start capturing all network traffic
    let capture_script = format!(r#"
        (function() {{
            const captureStart = Date.now();
            const captureDuration = {};

            // Override fetch for comprehensive capture
            const originalFetch = window.fetch;
            window.fetch = function(input, init) {{
                const url = typeof input === 'string' ? input : input.url;
                const method = init ? init.method : 'GET';
                const timestamp = Date.now();

                // Send capture event
                if (window.captureNetworkEvent) {{
                    window.captureNetworkEvent({{
                        type: 'request',
                        url: url,
                        method: method,
                        timestamp: timestamp
                    }});
                }}

                return originalFetch.apply(this, arguments)
                    .then(response => {{
                        if (window.captureNetworkEvent) {{
                            window.captureNetworkEvent({{
                                type: 'response',
                                url: url,
                                status: response.status,
                                statusText: response.statusText,
                                contentType: response.headers.get('content-type'),
                                timestamp: Date.now()
                            }});
                        }}
                        return response;
                    }})
                    .catch(error => {{
                        if (window.captureNetworkEvent) {{
                            window.captureNetworkEvent({{
                                type: 'error',
                                url: url,
                                error: error.message,
                                timestamp: Date.now()
                            }});
                        }}
                        throw error;
                    }});
            }};

            // Auto-stop after duration
            setTimeout(() => {{
                window.fetch = originalFetch;
                console.log('Network capture stopped after', captureDuration, 'ms');
            }}, captureDuration);

            return 'Network capture started';
        }})()
    "#, duration_ms);

    // Set up capture event handler
    let capture_handler = Closure::wrap(Box::new(move |event_data: JsValue| {
        if let Ok(event_obj) = event_data.dyn_into::<Object>() {
            let event_type = Reflect::get(&event_obj, &JsValue::from_str("type"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();

            let url = Reflect::get(&event_obj, &JsValue::from_str("url"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();

            let timestamp = Reflect::get(&event_obj, &JsValue::from_str("timestamp"))
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let network_event = match event_type.as_str() {
                "request" => {
                    let method = Reflect::get(&event_obj, &JsValue::from_str("method"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_else(|| "GET".to_string());

                    NetworkEvent::Request {
                        url,
                        method,
                        headers: HashMap::new(),
                        timestamp,
                    }
                },
                "response" => {
                    let status = Reflect::get(&event_obj, &JsValue::from_str("status"))
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as u16;

                    let status_text = Reflect::get(&event_obj, &JsValue::from_str("statusText"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_default();

                    let content_type = Reflect::get(&event_obj, &JsValue::from_str("contentType"))
                        .ok()
                        .and_then(|v| v.as_string());

                    NetworkEvent::Response {
                        url,
                        status,
                        status_text,
                        content_type,
                        size: 0, // TODO: Calculate actual size
                        duration: timestamp - Reflect::get(&event_obj, &JsValue::from_str("_requestTime"))
                            .ok()
                            .and_then(|v| v.as_f64())
                            .unwrap_or(timestamp),
                    }
                },
                "error" => {
                    let error = Reflect::get(&event_obj, &JsValue::from_str("error"))
                        .ok()
                        .and_then(|v| v.as_string())
                        .unwrap_or_else(|| "Unknown error".to_string());

                    NetworkEvent::Error {
                        url,
                        error,
                        timestamp,
                    }
                },
                _ => return,
            };

            let _ = tx.unbounded_send(network_event);
        }
    }) as Box<dyn FnMut(JsValue)>);

    Reflect::set(&js_sys::global(), &JsValue::from_str("captureNetworkEvent"), capture_handler.as_ref().unchecked_ref())
        .ok();

    let _ = js_sys::eval(&capture_script);

    // Wait for capture duration
    gloo_timers::future::TimeoutFuture::new(duration_ms as u32).await;

    // Clean up
    Reflect::delete_property(&js_sys::global(), &JsValue::from_str("captureNetworkEvent")).ok();

    // Collect all captured events
    let mut events = Vec::new();
    while let Ok(Some(event)) = rx.try_next() {
        events.push(event);
    }

    log::info!("Captured {} network events over {}ms", events.len(), duration_ms);
    Ok(events)
}
