//! JavaScript execution and dynamic agent injection capabilities
//!
//! This module provides comprehensive JavaScript evaluation, code injection,
//! context management, and event handler installation for browser automation.

use crate::Error;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window, Event, EventTarget, Function, Object};
use js_sys::{Array, Promise, Reflect};

/// JavaScript execution context
#[derive(Debug, Clone)]
pub struct JsExecutionContext {
    pub context_id: String,
    pub global_scope: Object,
    pub custom_functions: std::collections::HashMap<String, Function>,
    pub event_listeners: Vec<String>,
}

/// JavaScript execution result
#[derive(Debug, Clone)]
pub struct JsResult {
    pub value: JsValue,
    pub execution_time_ms: u64,
    pub has_errors: bool,
    pub output_log: Vec<String>,
}

/// Event handler configuration
#[derive(Debug, Clone)]
pub struct EventHandlerConfig {
    pub selector: String,
    pub event_type: String,
    pub handler_code: String,
    pub capture: bool,
    pub once: bool,
}

/// Execute JavaScript code with result capture
pub async fn execute_script(script: &str) -> Result<JsValue, Error> {
    let window = window().ok_or_else(|| Error::BrowserAutomation("No global window available".to_string()))?;

    // Use console.time for performance tracking
    let timer_id = format!("ia-js-{}", js_sys::Date::now());
    console::time_with_label(&timer_id);

    let result = js_sys::eval(script)
        .map_err(|err| {
            console::time_end_with_label(&timer_id);
            Error::BrowserAutomation(format!("JavaScript execution failed: {:?}", err))
        })?;

    console::time_end_with_label(&timer_id);
    log::info!("Executed JavaScript code ({} chars)", script.len());

    Ok(result)
}

/// Execute expression and evaluate result
pub async fn evaluate_expression(expression: &str) -> Result<JsValue, Error> {
    // Create a function that evaluates the expression in a controlled scope
    let script = format!(r#"
        (function() {{
            "use strict";
            try {{
                return {};
            }} catch (error) {{
                console.error("Infrastructure Assassin expression evaluation error:", error);
                return null;
            }}
        }})()
    "#, expression.trim().trim_end_matches(';'));

    execute_script(&script).await
}

/// Inject dynamic agent with JavaScript execution capabilities
pub async fn inject_dynamic_agent(js_code: &str) -> Result<(), Error> {
    let agent_wrapper = format!(r#"
        (function() {{
            "use strict";

            // Create Infrastructure Assassin namespace if it doesn't exist
            if (typeof window.infrastructureAssassin === 'undefined') {{
                window.infrastructureAssassin = {{
                    agents: new Map(),
                    sessionId: 'ia-{}',
                    version: '2.0.0'
                }};
            }}

            // Execute the provided agent code
            try {{
                {}
            }} catch (error) {{
                console.error("Infrastructure Assassin agent injection failed:", error);
                return false;
            }}

            console.log("Infrastructure Assassin dynamic agent injected successfully");
            return true;
        }})()
    "#, js_sys::Date::now(), js_code);

    let result = execute_script(&agent_wrapper).await?;

    if let Ok(success) = result.as_bool() {
        if success {
            log::info!("Successfully injected dynamic agent with {} chars of code", js_code.len());
            Ok(())
        } else {
            Err(Error::BrowserAutomation("Agent injection returned false".to_string()))
        }
    } else {
        Err(Error::BrowserAutomation("Agent injection did not return boolean result".to_string()))
    }
}

/// Install custom event handlers on DOM elements
pub async fn install_event_handlers() -> Result<(), Error> {
    let handlers = vec![
        EventHandlerConfig {
            selector: "input[type='text'], input[type='password'], textarea",
            event_type: "focus",
            handler_code: r#"
                if (!event.target.hasAttribute('data-ia-tracked')) {
                    event.target.setAttribute('data-ia-tracked', 'true');
                    console.log('Infrastructure Assassin: Input element tracked:', event.target.tagName, event.target.type || '');
                }
            "#.to_string(),
            capture: true,
            once: false,
        },
        EventHandlerConfig {
            selector: "form",
            event_type: "submit",
            handler_code: r#"
                console.log('Infrastructure Assassin: Form submission detected');
                window.infrastructureAssassin = window.infrastructureAssassin || { sessionId: 'ia-' + Date.now() };
                window.infrastructureAssassin.lastFormSubmit = new Date().toISOString();
            "#.to_string(),
            capture: true,
            once: false,
        },
        EventHandlerConfig {
            selector: "a[href], button, [role='button']",
            event_type: "click",
            handler_code: r#"
                console.log('Infrastructure Assassin: Interactive element clicked:', event.target.tagName, event.target.innerText || event.target.textContent || '');
            "#.to_string(),
            capture: true,
            once: false,
        },
    ];

    for config in handlers {
        install_single_event_handler(&config).await?;
    }

    log::info!("Installed {} Infrastructure Assassin event handlers", handlers.len());
    Ok(())
}

/// Install a single event handler
pub async fn install_single_event_handler(config: &EventHandlerConfig) -> Result<(), Error> {
    let window = window().ok_or_else(|| Error::BrowserAutomation("No global window available".to_string()))?;
    let document = window.document()
        .ok_or_else(|| Error::BrowserAutomation("No document available".to_string()))?;

    let elements = document.query_selector_all(&config.selector)
        .map_err(|_| Error::BrowserAutomation(format!("Failed to query selector: {}", config.selector)))?;

    for i in 0..elements.length() {
        if let Ok(element) = elements.get(i).dyn_into::<EventTarget>() {
            let handler_code = config.handler_code.clone();

            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: Event| {
                let script = format!("(function(event) {{ {} }})(arguments[0])", handler_code);
                if let Ok(result) = js_sys::eval(&script) {
                    // Handler executed successfully
                } else {
                    console::error_1(&JsValue::from_str("Infrastructure Assassin event handler error"));
                }
            }) as Box<dyn FnMut(Event)>);

            element.add_event_listener_with_event_listener_and_add_event_listener_options(
                &config.event_type,
                closure.as_ref().unchecked_ref(),
                &web_sys::AddEventListenerOptions::new()
                    .capture(config.capture)
                    .once(config.once)
            ).map_err(|_| Error::BrowserAutomation("Failed to add event listener".to_string()))?;

            closure.forget(); // Keep alive
        }
    }

    log::debug!("Installed {} event handlers for selector: {}", elements.length(), config.selector);
    Ok(())
}

/// Create a new JavaScript execution context
pub fn create_js_context() -> Result<JsExecutionContext, Error> {
    let context_id = format!("ia-js-ctx-{}", js_sys::Date::now());

    // Create global scope object for the context
    let global_scope = Object::new();

    // Initialize Infrastructure Assassin global namespace
    let ia_ns = Object::new();
    Reflect::set(&ia_ns, &JsValue::from_str("contextId"), &JsValue::from_str(&context_id))
        .map_err(|_| Error::BrowserAutomation("Failed to set context ID".to_string()))?;
    Reflect::set(&ia_ns, &JsValue::from_str("version"), &JsValue::from_str("2.0.0"))
        .map_err(|_| Error::BrowserAutomation("Failed to set version".to_string()))?;

    if let Ok(window) = window() {
        Reflect::set(&window, &JsValue::from_str("infrastructureAssassin"), &ia_ns)
            .map_err(|_| Error::BrowserAutomation("Failed to set global namespace".to_string()))?;
    }

    let context = JsExecutionContext {
        context_id,
        global_scope: ia_ns,
        custom_functions: std::collections::HashMap::new(),
        event_listeners: Vec::new(),
    };

    log::info!("Created JavaScript execution context: {}", context_id);
    Ok(context)
}

/// Execute JavaScript in isolated context
pub async fn execute_in_context(context: &JsExecutionContext, script: &str) -> Result<JsValue, Error> {
    // Modify script to run within the Infrastructure Assassin context
    let wrapped_script = format!(r#"
        (function() {{
            "use strict";
            var ia = window.infrastructureAssassin;

            try {{
                {}
            }} catch (error) {{
                console.error("Infrastructure Assassin context execution error:", error);
                throw error;
            }}
        }})()
    "#, script);

    execute_script(&wrapped_script).await
}

/// Inject browser automation utilities
pub async fn inject_browser_utilities() -> Result<(), Error> {
    let utilities_code = r#"
        (function() {
            if (window.infrastructureAssassin && window.infrastructureAssassin.utils) {
                return; // Already injected
            }

            window.infrastructureAssassin = window.infrastructureAssassin || {};
            window.infrastructureAssassin.utils = {
                // Get element by improved selector matching
                getElementByText: function(text, tag = "*") {
                    const elements = document.querySelectorAll(tag);
                    for (let el of elements) {
                        if (el.textContent && el.textContent.trim().includes(text)) {
                            return el;
                        }
                    }
                    return null;
                },

                // Wait for element to appear
                waitForElement: function(selector, timeout = 5000) {
                    return new Promise((resolve, reject) => {
                        const element = document.querySelector(selector);
                        if (element) {
                            resolve(element);
                            return;
                        }

                        const observer = new MutationObserver(() => {
                            const element = document.querySelector(selector);
                            if (element) {
                                observer.disconnect();
                                resolve(element);
                            }
                        });

                        observer.observe(document.body, {
                            childList: true,
                            subtree: true
                        });

                        setTimeout(() => {
                            observer.disconnect();
                            reject(new Error(`Element ${selector} not found within ${timeout}ms`));
                        }, timeout);
                    });
                },

                // Simulate human-like typing
                typeText: function(element, text, delay = 100) {
                    return new Promise(resolve => {
                        let i = 0;
                        const typeChar = () => {
                            if (i < text.length) {
                                element.value += text[i];
                                element.dispatchEvent(new Event('input', { bubbles: true }));
                                i++;
                                setTimeout(typeChar, delay);
                            } else {
                                resolve();
                            }
                        };
                        typeChar();
                    });
                },

                // Get page analytics data
                getPageAnalytics: function() {
                    return {
                        url: window.location.href,
                        title: document.title,
                        forms: document.forms.length,
                        links: document.links.length,
                        images: document.images.length,
                        scripts: document.scripts.length,
                        stylesheets: document.styleSheets.length,
                        timestamp: new Date().toISOString()
                    };
                },

                // Inject CSS for visualization
                injectStyles: function(css) {
                    const style = document.createElement('style');
                    style.textContent = css;
                    document.head.appendChild(style);
                    return style;
                }
            };

            console.log('Infrastructure Assassin browser utilities injected');
        })()
    "#;

    inject_dynamic_agent(utilities_code).await
}

/// Monitor JavaScript console output
pub async fn monitor_console_output() -> Result<(), Error> {
    // Override console methods to capture output
    let monitor_code = r#"
        (function() {
            const originalLog = console.log;
            const originalError = console.error;
            const originalWarn = console.warn;
            const originalInfo = console.info;

            const iaConsole = window.infrastructureAssassin.console = {
                logs: [],
                maxLogs: 1000
            };

            function captureLog(level, args) {
                const entry = {
                    level: level,
                    timestamp: new Date().toISOString(),
                    message: Array.from(args).join(' '),
                    sessionId: window.infrastructureAssassin.sessionId
                };

                iaConsole.logs.push(entry);
                if (iaConsole.logs.length > iaConsole.maxLogs) {
                    iaConsole.logs.shift();
                }

                // Call original method
                if (level === 'log') originalLog.apply(console, args);
                else if (level === 'error') originalError.apply(console, args);
                else if (level === 'warn') originalWarn.apply(console, args);
                else if (level === 'info') originalInfo.apply(console, args);
            }

            console.log = function() { captureLog('log', arguments); };
            console.error = function() { captureLog('error', arguments); };
            console.warn = function() { captureLog('warn', arguments); };
            console.info = function() { captureLog('info', arguments); };

            console.log('Infrastructure Assassin console monitoring active');
        })()
    "#;

    inject_dynamic_agent(monitor_code).await
}

/// Execute JavaScript with performance timing
pub async fn execute_with_performance(script: &str) -> Result<JsResult, Error> {
    let start_time = web_sys::Performance::now() as u64;

    // Capture console output before execution if monitoring is active
    let console_capture = if let Ok(window) = window() {
        if let Ok(ia_ns) = Reflect::get(&window, &JsValue::from_str("infrastructureAssassin")) {
            if let Ok(console_obj) = Reflect::get(&ia_ns, &JsValue::from_str("console")) {
                if let Ok(logs) = Reflect::get(&console_obj, &JsValue::from_str("logs")) {
                    logs
                } else {
                    JsValue::NULL
                }
            } else {
                JsValue::NULL
            }
        } else {
            JsValue::NULL
        }
    } else {
        JsValue::NULL
    };

    let initial_log_count = if console_capture.is_object() {
        if let Ok(array) = console_capture.dyn_into::<Array>() {
            array.length() as usize
        } else {
            0
        }
    } else {
        0
    };

    let result = execute_script(script).await;
    let execution_time = web_sys::Performance::now() as u64 - start_time;

    let (value, has_errors) = match result {
        Ok(value) => (value, false),
        Err(err) => (JsValue::from_str(&format!("{:?}", err)), true),
    };

    // Extract console output after execution
    let output_log = Vec::new(); // TODO: Extract from captured logs

    Ok(JsResult {
        value,
        execution_time_ms: execution_time,
        has_errors,
        output_log,
    })
}
