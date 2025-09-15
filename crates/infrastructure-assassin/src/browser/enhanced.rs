//! Enhanced DOM manipulation and page control capabilities
//!
//! This module provides real-time browser automation through direct DOM access,
//! element manipulation, content extraction, and dynamic agent injection.

use crate::{Error, BrowserSession};
use futures::Stream;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    console, window, CanvasRenderingContext2d, Document, Element, Event, EventTarget,
    HtmlCanvasElement, HtmlElement, MutationObserver, MutationObserverInit, MutationRecord,
    Node,
};

/// Enhanced browser session with full DOM capabilities
pub struct RealBrowserSession {
    pub session_id: String,
    pub config: crate::browser::BrowserConfig,
    pub dom_handles: std::collections::HashMap<String, Element>,
    pub observers: Vec<MutationObserver>,
}

/// DOM events for real-time monitoring
#[derive(Debug, Clone)]
pub enum DomEvent {
    ElementAdded { selector: String, element: Element },
    ElementRemoved { selector: String },
    AttributeChanged { selector: String, attribute: String, old_value: Option<String>, new_value: Option<String> },
    ContentChanged { selector: String, old_content: String, new_content: String },
}

/// DOM manipulation actions
#[derive(Debug, Clone)]
pub enum DomAction {
    Click,
    Type { text: String },
    SetAttribute { name: String, value: String },
    RemoveAttribute { name: String },
    SetText { content: String },
    AppendChild { html: String },
    RemoveChild { selector: String },
}

/// Agent configuration for dynamic injection
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentConfig {
    pub agent_type: String,
    pub capabilities: Vec<String>,
    pub selectors: std::collections::HashMap<String, String>,
    pub event_handlers: Vec<String>,
}

/// Extract complete page content (HTML/text/CSS)
pub async fn extract_page_content() -> Result<String, Error> {
    let window = window().ok_or_else(|| Error::BrowserAutomation("No global window available".to_string()))?;
    let document = window.document()
        .ok_or_else(|| Error::BrowserAutomation("No document available".to_string()))?;

    // Extract HTML content
    let html_content = document.document_element()
        .ok_or_else(|| Error::BrowserAutomation("No document element found".to_string()))?
        .outer_html();

    // Extract all text content for processing
    let body = document.body()
        .ok_or_else(|| Error::BrowserAutomation("No body element found".to_string()))?;
    let text_content = body.text_content()
        .unwrap_or_default();

    // Extract computed styles (basic implementation)
    let all_elements = document.query_selector_all("*")
        .map_err(|_| Error::BrowserAutomation("Failed to query all elements".to_string()))?;

    let mut styles_content = String::new();
    for i in 0..all_elements.length() {
        if let Ok(element) = all_elements.get(i).dyn_into::<Element>() {
            if let Ok(computed_style) = window.get_computed_style(&element) {
                if let Some(style) = computed_style {
                    styles_content.push_str(&format!("[data-element-id='{}'] {{ {} }}\n",
                        i, style.css_text()));
                }
            }
        }
    }

    let content = format!("<!-- INFRASTRUCTURE ASSASSIN PAGE EXTRACTION -->\n\
                         <html_content>\n{}\n</html_content>\n\
                         <text_content>\n{}\n</text_content>\n\
                         <computed_styles>\n{}\n</computed_styles>",
                         html_content, text_content, styles_content);

    log::info!("Extracted {} characters of page content", content.len());
    Ok(content)
}

/// Monitor DOM changes in real-time
pub async fn monitor_dom_changes(selector: &str) -> Result<impl Stream<Item = DomEvent>, Error> {
    let window = window().ok_or_else(|| Error::BrowserAutomation("No global window available".to_string()))?;
    let document = window.document()
        .ok_or_else(|| Error::BrowserAutomation("No document available".to_string()))?;

    let target_element = document.query_selector(selector)
        .map_err(|_| Error::BrowserAutomation(format!("Selector '{}' not found", selector)))?
        .ok_or_else(|| Error::BrowserAutomation(format!("Element with selector '{}' not found", selector)))?;

    let (tx, rx) = futures::channel::mpsc::unbounded();

    let observer_callback = wasm_bindgen::closure::Closure::wrap(Box::new(move |mutations: Vec<JsValue>, _observer: JsValue| {
        for mutation_js in mutations {
            if let Ok(mutation) = mutation_js.dyn_into::<MutationRecord>() {
                // Handle different mutation types
                if mutation.type_() == "childList" {
                    // Element added/removed
                    let added_nodes = mutation.added_nodes();
                    for i in 0..added_nodes.length() {
                        if let Ok(node) = added_nodes.get(i).dyn_into::<Node>() {
                            if let Ok(element) = node.dyn_into::<Element>() {
                                let event = DomEvent::ElementAdded {
                                    selector: selector.to_string(),
                                    element: element.clone(),
                                };
                                let _ = tx.unbounded_send(event);
                            }
                        }
                    }

                    let removed_nodes = mutation.removed_nodes();
                    for i in 0..removed_nodes.length() {
                        if let Ok(_node) = removed_nodes.get(i).dyn_into::<Node>() {
                            let event = DomEvent::ElementRemoved {
                                selector: selector.to_string(),
                            };
                            let _ = tx.unbounded_send(event);
                        }
                    }
                } else if mutation.type_() == "attributes" {
                    // Attribute changed
                    let attribute_name = mutation.attribute_name().unwrap_or_default();
                    let old_value = mutation.old_value();
                    let new_value = mutation.attribute_namespace().unwrap_or_else(|_| JsValue::NULL);

                    let event = DomEvent::AttributeChanged {
                        selector: selector.to_string(),
                        attribute: attribute_name,
                        old_value: old_value.as_string(),
                        new_value: new_value.as_string(),
                    };
                    let _ = tx.unbounded_send(event);
                } else if mutation.type_() == "characterData" {
                    // Text content changed
                    let old_value = mutation.old_value().unwrap_or_else(|_| JsValue::from(""));
                    let target = mutation.target();

                    let event = DomEvent::ContentChanged {
                        selector: selector.to_string(),
                        old_content: old_value.as_string().unwrap_or_default(),
                        new_content: target.text_content().unwrap_or_default(),
                    };
                    let _ = tx.unbounded_send(event);
                }
            }
        }
    }) as Box<dyn FnMut(Vec<JsValue>, JsValue)>);

    let mut init = MutationObserverInit::new();
    init.child_list(true);
    init.attributes(true);
    init.attribute_old_value(true);
    init.character_data(true);
    init.character_data_old_value(true);
    init.subtree(true);

    let observer = MutationObserver::new(&observer_callback.as_ref().unchecked_ref())
        .map_err(|_| Error::BrowserAutomation("Failed to create mutation observer".to_string()))?;

    observer.observe_with_options(&target_element, &init)
        .map_err(|_| Error::BrowserAutomation("Failed to start DOM monitoring".to_string()))?;

    // Keep the callback alive
    observer_callback.forget();

    // Return stream that yields DOM events
    Ok(rx)
}

/// Perform DOM manipulation actions
pub async fn manipulate_dom(selector: &str, action: DomAction) -> Result<(), Error> {
    let window = window().ok_or_else(|| Error::BrowserAutomation("No global window available".to_string()))?;
    let document = window.document()
        .ok_or_else(|| Error::BrowserAutomation("No document available".to_string()))?;

    let element = document.query_selector(selector)
        .map_err(|_| Error::BrowserAutomation(format!("Failed to query selector: {}", selector)))?
        .ok_or_else(|| Error::BrowserAutomation(format!("Element not found: {}", selector)))?;

    match action {
        DomAction::Click => {
            if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                html_element.click();
                log::info!("Clicked element: {}", selector);
            }
        },
        DomAction::Type { text } => {
            // Simulate typing by setting value and triggering input events
            if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                html_element.set_attribute("value", &text)
                    .map_err(|_| Error::BrowserAutomation("Failed to set input value".to_string()))?;

                // Trigger input event
                let event = Event::new("input")
                    .map_err(|_| Error::BrowserAutomation("Failed to create input event".to_string()))?;
                html_element.dispatch_event(&event)
                    .map_err(|_| Error::BrowserAutomation("Failed to dispatch input event".to_string()))?;

                log::info!("Typed '{}' into element: {}", text, selector);
            }
        },
        DomAction::SetAttribute { name, value } => {
            element.set_attribute(&name, &value)
                .map_err(|_| Error::BrowserAutomation(format!("Failed to set attribute {}={}", name, value)))?;
            log::info!("Set attribute {}='{}' on element: {}", name, value, selector);
        },
        DomAction::RemoveAttribute { name } => {
            element.remove_attribute(&name)
                .map_err(|_| Error::BrowserAutomation(format!("Failed to remove attribute {}", name)))?;
            log::info!("Removed attribute '{}' from element: {}", name, selector);
        },
        DomAction::SetText { content } => {
            element.set_text_content(Some(&content));
            log::info!("Set text content on element: {}", selector);
        },
        DomAction::AppendChild { html } => {
            let fragment = document.create_element("div")
                .map_err(|_| Error::BrowserAutomation("Failed to create element".to_string()))?;
            fragment.set_inner_html(&html);

            while let Ok(Some(child)) = fragment.first_child() {
                element.append_child(&child)
                    .map_err(|_| Error::BrowserAutomation("Failed to append child".to_string()))?;
            }
            log::info!("Appended HTML to element: {}", selector);
        },
        DomAction::RemoveChild { child_selector } => {
            let child = element.query_selector(&child_selector)
                .map_err(|_| Error::BrowserAutomation(format!("Failed to find child: {}", child_selector)))?
                .ok_or_else(|| Error::BrowserAutomation(format!("Child element not found: {}", child_selector)))?;

            element.remove_child(&child)
                .map_err(|_| Error::BrowserAutomation("Failed to remove child".to_string()))?;
            log::info!("Removed child from element: {}", selector);
        }
    }

    Ok(())
}

/// Inject agent UI into live web pages
pub async fn inject_agent_ui(selector: &str, agent_config: AgentConfig) -> Result<(), Error> {
    let html_content = format!(r#"
        <div id="infrastructure-assassin-{}" style="
            position: fixed;
            top: 20px;
            right: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 15px;
            border-radius: 12px;
            box-shadow: 0 8px 32px rgba(0,0,0,0.1);
            z-index: 999999;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            font-size: 14px;
            max-width: 300px;
            backdrop-filter: blur(10px);
        ">
            <div style="display: flex; align-items: center; margin-bottom: 10px;">
                <div style="width: 8px; height: 8px; border-radius: 50%; background: #00ff88; margin-right: 8px; animation: pulse 2s infinite;"></div>
                <strong>Infrastructure Assassin</strong>
            </div>
            <div style="margin-bottom: 8px;">Type: <code style="background: rgba(255,255,255,0.1); padding: 2px 6px; border-radius: 4px;">{}</code></div>
            <div>Capabilities: {}</div>
            <div style="margin-top: 10px; font-size: 12px; opacity: 0.8;">Session ID: {}</div>
            <style>
                @keyframes pulse {{
                    0%, 100% {{ opacity: 1; }}
                    50% {{ opacity: 0.5; }}
                }}
            </style>
        </div>
    "#, agent_config.agent_type.to_lowercase().replace(" ", "-"), agent_config.agent_type, agent_config.capabilities.join(", "), generate_session_id());

    manipulate_dom(selector, DomAction::AppendChild { html: html_content }).await?;

    log::info!("Injected agent UI for '{}' with {} capabilities", agent_config.agent_type, agent_config.capabilities.len());
    Ok(())
}

/// Create a real browser session with full DOM capabilities
pub fn create_real_browser_session(config: crate::browser::BrowserConfig) -> Result<RealBrowserSession, Error> {
    let session_id = format!("infrastructure-assassin-real-{}", js_sys::Date::now());

    let session = RealBrowserSession {
        session_id: session_id.clone(),
        config,
        dom_handles: std::collections::HashMap::new(),
        observers: Vec::new(),
    };

    log::info!("Created real browser session: {}", session_id);
    Ok(session)
}

/// Generate unique session identifier
fn generate_session_id() -> String {
    format!("ia-{}", js_sys::Date::now())
}

/// Get element by selector for direct manipulation
pub fn get_element_by_selector(selector: &str) -> Result<Element, Error> {
    let document = window()
        .ok_or_else(|| Error::BrowserAutomation("No global window available".to_string()))?
        .document()
        .ok_or_else(|| Error::BrowserAutomation("No document available".to_string()))?;

    document.query_selector(selector)
        .map_err(|_| Error::BrowserAutomation(format!("Query failed for selector: {}", selector)))?
        .ok_or_else(|| Error::BrowserAutomation(format!("Element not found: {}", selector)))
}
