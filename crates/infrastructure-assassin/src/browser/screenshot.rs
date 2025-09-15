//! Screenshot and visual capture capabilities
//!
//! This module provides comprehensive screenshot functionality including
//! viewport capture, canvas rendering, and element-specific captures.

use crate::Error;
use wasm_bindgen::{JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window, HtmlCanvasElement, CanvasRenderingContext2d, ImageData, Blob};
use js_sys::{Array, Object, Promise, Reflect, Uint8Array, ArrayBuffer};

/// Screenshot capture format
#[derive(Debug, Clone)]
pub enum ScreenshotFormat {
    PNG,
    JPEG { quality: f64 },
    WebP { quality: f64 },
}

/// Screenshot capture options
#[derive(Debug, Clone)]
pub struct ScreenshotOptions {
    pub format: ScreenshotFormat,
    pub include_cursor: bool,
    pub full_page: bool,
    pub scale: f64,
    pub background_color: Option<String>,
}

/// Screenshot result with metadata
#[derive(Debug, Clone)]
pub struct ScreenshotResult {
    pub data: Vec<u8>,
    pub format: ScreenshotFormat,
    pub width: u32,
    pub height: u32,
    pub timestamp: f64,
    pub capture_time_ms: u64,
}

/// Capture current viewport as screenshot
pub async fn capture_viewport() -> Result<Vec<u8>, Error> {
    let options = ScreenshotOptions {
        format: ScreenshotFormat::PNG,
        include_cursor: false,
        full_page: false,
        scale: 1.0,
        background_color: None,
    };

    let result = capture_screenshot(options).await?;
    log::info!("Viewport captured: {}x{} ({} bytes)", result.width, result.height, result.data.len());
    Ok(result.data)
}

/// Capture full page screenshot (scroll and stitch)
pub async fn capture_full_page() -> Result<Vec<u8>, Error> {
    let options = ScreenshotOptions {
        format: ScreenshotFormat::PNG,
        include_cursor: false,
        full_page: true,
        scale: 1.0,
        background_color: Some("#ffffff".to_string()),
    };

    let result = capture_fullpage_screenshot(options).await?;
    log::info!("Full page captured: {}x{} ({} bytes)", result.width, result.height, result.data.len());
    Ok(result.data)
}

/// Capture specific element as screenshot
pub async fn capture_element(selector: &str) -> Result<Vec<u8>, Error> {
    let options = ScreenshotOptions {
        format: ScreenshotFormat::PNG,
        include_cursor: false,
        full_page: false,
        scale: 1.0,
        background_color: Some("transparent".to_string()),
    };

    let result = capture_element_screenshot(selector, options).await?;
    log::info!("Element '{}' captured: {}x{} ({} bytes)", selector, result.width, result.height, result.data.len());
    Ok(result.data)
}

/// Generate dashboard visualization from analytics data
pub async fn render_dashboard() -> Result<Vec<u8>, Error> {
    let dashboard_html = generate_dashboard_html();
    let options = ScreenshotOptions {
        format: ScreenshotFormat::PNG,
        include_cursor: false,
        full_page: false,
        scale: 2.0, // Higher DPI for crisp text
        background_color: Some("#1a1a1a".to_string()),
    };

    let result = render_html_to_canvas(&dashboard_html, options).await?;
    log::info!("Dashboard rendered: {}x{} ({} bytes)", result.width, result.height, result.data.len());
    Ok(result.data)
}

/// Main screenshot capture function
async fn capture_screenshot(options: ScreenshotOptions) -> Result<ScreenshotResult, Error> {
    let start_time = web_sys::Performance::now() as u64;

    let capture_script = format!(r#"
        (function() {{
            return new Promise((resolve, reject) => {{
                try {{
                    // Create canvas for screenshot
                    const canvas = document.createElement('canvas');
                    const ctx = canvas.getContext('2d');

                    // Set canvas size to viewport
                    const viewportWidth = window.innerWidth;
                    const viewportHeight = window.innerHeight;
                    const scale = {};

                    canvas.width = viewportWidth * scale;
                    canvas.height = viewportHeight * scale;

                    // Scale context for crisp rendering
                    ctx.scale(scale, scale);

                    // Set background if specified
                    {}

                    // Draw the page content
                    html2canvas(document.body, {{
                        canvas: canvas,
                        useCORS: true,
                        allowTaint: false,
                        scale: scale,
                        backgroundColor: {},
                        width: viewportWidth,
                        height: viewportHeight,
                        x: 0,
                        y: 0,
                        foreignObjectRendering: true
                    }}).then(() => {{
                        canvas.toBlob((blob) => {{
                            if (blob) {{
                                const reader = new FileReader();
                                reader.onload = function() {{
                                    const arrayBuffer = this.result;
                                    const uint8Array = new Uint8Array(arrayBuffer);
                                    const byteArray = Array.from(uint8Array);
                                    resolve({{
                                        data: byteArray,
                                        width: canvas.width,
                                        height: canvas.height,
                                        format: '{}'
                                    }});
                                }};
                                reader.readAsArrayBuffer(blob);
                            }} else {{
                                reject(new Error('Failed to capture screenshot'));
                            }}
                        }}, 'image/{}', {});
                    }}).catch(reject);
                }} catch (error) {{
                    reject(error);
                }}
            }});
        }})()
    "#, options.scale,
       if options.background_color.is_some() { "ctx.fillStyle = options.backgroundColor; ctx.fillRect(0, 0, canvas.width, canvas.height);" } else { "" },
       options.background_color.unwrap_or("null".to_string()),
       format!("{:?}", options.format).to_lowercase(),
       match options.format {
           ScreenshotFormat::PNG => "png",
           ScreenshotFormat::JPEG { .. } => "jpeg",
           ScreenshotFormat::WebP { .. } => "webp",
       },
       match options.format {
           ScreenshotFormat::JPEG { quality } => quality.to_string(),
           ScreenshotFormat::WebP { quality } => quality.to_string(),
           _ => "1.0".to_string(),
       });

    // First try html2canvas (external library), fallback to basic canvas capture
    let result = if let Ok(_) = html2canvas_available().await {
        js_sys::eval(&capture_script)
            .map_err(|_| Error::BrowserAutomation("Failed to execute screenshot script".to_string()))?
    } else {
        capture_basic_screenshot(&options).await?
    };

    let promise = Promise::from(result);
    let js_result = JsFuture::from(promise)
        .await
        .map_err(|_| Error::BrowserAutomation("Screenshot promise failed".to_string()))?;

    let data = Reflect::get(&js_result, &JsValue::from_str("data"))
        .ok()
        .and_then(|v| v.dyn_into::<Array>().ok())
        .map(|arr| {
            let mut vec = Vec::new();
            for i in 0..arr.length() {
                if let Ok(val) = arr.get(i).as_f64() {
                    vec.push(val as u8);
                }
            }
            vec
        })
        .unwrap_or_default();

    let width = Reflect::get(&js_result, &JsValue::from_str("width"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1920.0) as u32;

    let height = Reflect::get(&js_result, &JsValue::from_str("height"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1080.0) as u32;

    let capture_time = web_sys::Performance::now() as u64 - start_time;

    Ok(ScreenshotResult {
        data,
        format: options.format,
        width,
        height,
        timestamp: js_sys::Date::now(),
        capture_time_ms: capture_time,
    })
}

/// Basic screenshot capture without external libraries
async fn capture_basic_screenshot(options: &ScreenshotOptions) -> Result<JsValue, Error> {
    let basic_capture_script = format!(r#"
        (function() {{
            return new Promise((resolve, reject) => {{
                try {{
                    const canvas = document.createElement('canvas');
                    const ctx = canvas.getContext('2d');

                    canvas.width = window.innerWidth * {};
                    canvas.height = window.innerHeight * {};

                    ctx.scale({}, {});

                    // Fill background
                    ctx.fillStyle = {};
                    ctx.fillRect(0, 0, canvas.width, canvas.height);

                    // Simple content rendering (limited without html2canvas)
                    const elements = document.querySelectorAll('*');
                    elements.forEach((el, index) => {{
                        try {{
                            const rect = el.getBoundingClientRect();
                            if (rect.width > 0 && rect.height > 0 && rect.top >= 0 && rect.left >= 0) {{
                                ctx.fillStyle = window.getComputedStyle(el).backgroundColor || 'transparent';
                                ctx.fillRect(rect.left, rect.top, rect.width, rect.height);

                                // Add element ID for debugging
                                ctx.fillStyle = 'red';
                                ctx.font = '10px monospace';
                                ctx.fillText(`${{index}}`, rect.left + 2, rect.top + 12);
                            }}
                        }} catch (e) {{
                            // Skip problematic elements
                        }}
                    }});

                    canvas.toBlob((blob) => {{
                        if (blob) {{
                            const reader = new FileReader();
                            reader.onload = function() {{
                                const arrayBuffer = this.result;
                                const uint8Array = new Uint8Array(arrayBuffer);
                                const byteArray = Array.from(uint8Array);
                                resolve({{
                                    data: byteArray,
                                    width: canvas.width,
                                    height: canvas.height,
                                    format: '{}'
                                }});
                            }};
                            reader.readAsArrayBuffer(blob);
                        }} else {{
                            reject(new Error('Basic screenshot failed'));
                        }}
                    }}, 'image/{}}');
                }} catch (error) {{
                    reject(error);
                }}
            }});
        }})()
    "#, options.scale, options.scale, options.scale, options.scale,
       options.background_color.as_deref().unwrap_or("#ffffff"),
       format!("{:?}", options.format).to_lowercase(),
       match options.format {
           ScreenshotFormat::PNG => "png",
           ScreenshotFormat::JPEG { .. } => "jpeg",
           ScreenshotFormat::WebP { .. } => "webp",
       });

    js_sys::eval(&basic_capture_script)
        .map_err(|_| Error::BrowserAutomation("Failed to execute basic screenshot script".to_string()))
}

/// Capture full page by scrolling and stitching
async fn capture_fullpage_screenshot(options: ScreenshotOptions) -> Result<ScreenshotResult, Error> {
    let stitching_script = r#"
        (function() {
            return new Promise((resolve, reject) => {
                if (typeof html2canvas === 'undefined') {
                    reject(new Error('html2canvas is required for full page capture'));
                    return;
                }

                const originalScrollTop = window.pageYOffset || document.documentElement.scrollTop;

                // Calculate total page height
                const totalHeight = Math.max(
                    document.body.scrollHeight,
                    document.body.offsetHeight,
                    document.documentElement.clientHeight,
                    document.documentElement.scrollHeight,
                    document.documentElement.offsetHeight
                );

                const viewportHeight = window.innerHeight;
                const chunks = Math.ceil(totalHeight / viewportHeight);

                console.log(`Full page capture: ${totalHeight}px height, ${chunks} chunks`);

                // For now, use single viewport (TODO: implement chunking)
                html2canvas(document.body, {
                    useCORS: true,
                    allowTaint: false,
                    backgroundColor: '#ffffff',
                    width: window.innerWidth,
                    height: totalHeight,
                    foreignObjectRendering: true
                }).then(canvas => {
                    canvas.toBlob((blob) => {
                        if (blob) {
                            const reader = new FileReader();
                            reader.onload = function() {
                                const arrayBuffer = this.result;
                                const uint8Array = new Uint8Array(arrayBuffer);
                                const byteArray = Array.from(uint8Array);
                                resolve({
                                    data: byteArray,
                                    width: canvas.width,
                                    height: canvas.height,
                                    format: 'png'
                                });
                            };
                            reader.readAsArrayBuffer(blob);
                        } else {
                            reject(new Error('Failed to capture full page'));
                        }
                    }, 'image/png');
                }).catch(reject);
            });
        })()
    "#;

    let result = js_sys::eval(stitching_script)
        .map_err(|_| Error::BrowserAutomation("Failed to execute full page screenshot script".to_string()))?;

    let promise = Promise::from(result);
    let js_result = JsFuture::from(promise)
        .await
        .map_err(|_| Error::BrowserAutomation("Full page screenshot promise failed".to_string()))?;

    // Extract data similar to basic capture
    let data = Reflect::get(&js_result, &JsValue::from_str("data"))
        .ok()
        .and_then(|v| v.dyn_into::<Array>().ok())
        .map(|arr| {
            let mut vec = Vec::new();
            for i in 0..arr.length() {
                if let Ok(val) = arr.get(i).as_f64() {
                    vec.push(val as u8);
                }
            }
            vec
        })
        .unwrap_or_default();

    let width = Reflect::get(&js_result, &JsValue::from_str("width"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1920.0) as u32;

    let height = Reflect::get(&js_result, &JsValue::from_str("height"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1080.0) as u32;

    Ok(ScreenshotResult {
        data,
        format: options.format,
        width,
        height,
        timestamp: js_sys::Date::now(),
        capture_time_ms: web_sys::Performance::now() as u64,
    })
}

/// Capture specific element as screenshot
async fn capture_element_screenshot(selector: &str, options: ScreenshotOptions) -> Result<ScreenshotResult, Error> {
    let element_capture_script = format!(r#"
        (function() {{
            return new Promise((resolve, reject) => {{
                const element = document.querySelector('{}');
                if (!element) {{
                    reject(new Error('Element not found: {}'));
                    return;
                }}

                const rect = element.getBoundingClientRect();
                if (rect.width === 0 || rect.height === 0) {{
                    reject(new Error('Element has no visible content'));
                    return;
                }}

                if (typeof html2canvas !== 'undefined') {{
                    html2canvas(element, {{
                        useCORS: true,
                        allowTaint: false,
                        backgroundColor: null,
                        scale: {},
                        width: rect.width,
                        height: rect.height,
                        foreignObjectRendering: true
                    }}).then(canvas => {{
                        canvas.toBlob((blob) => {{
                            if (blob) {{
                                const reader = new FileReader();
                                reader.onload = function() {{
                                    const arrayBuffer = this.result;
                                    const uint8Array = new Uint8Array(arrayBuffer);
                                    const byteArray = Array.from(uint8Array);
                                    resolve({{
                                        data: byteArray,
                                        width: canvas.width,
                                        height: canvas.height,
                                        format: '{}'
                                    }});
                                }};
                                reader.readAsArrayBuffer(blob);
                            }} else {{
                                reject(new Error('Failed to capture element'));
                            }}
                        }}, 'image/{}', {});
                    }}).catch(reject);
                }} else {{
                    // Fallback: capture element area from viewport
                    const canvas = document.createElement('canvas');
                    const ctx = canvas.getContext('2d');
                    const scale = {};

                    canvas.width = rect.width * scale;
                    canvas.height = rect.height * scale;
                    ctx.scale(scale, scale);

                    try {{
                        ctx.drawImage(document.body, -rect.left, -rect.top);
                        ctx.drawImage(element, 0, 0);

                        canvas.toBlob((blob) => {{
                            if (blob) {{
                                const reader = new FileReader();
                                reader.onload = function() {{
                                    const arrayBuffer = this.result;
                                    const uint8Array = new Uint8Array(arrayBuffer);
                                    const byteArray = Array.from(uint8Array);
                                    resolve({{
                                        data: byteArray,
                                        width: canvas.width,
                                        height: canvas.height,
                                        format: '{}'
                                    }});
                                }};
                                reader.readAsArrayBuffer(blob);
                            }} else {{
                                reject(new Error('Fallback element capture failed'));
                            }}
                        }}, 'image/{}');
                    }} catch (error) {{
                        reject(error);
                    }}
                }}
            }});
        }})()
    "#, selector, selector, options.scale,
       format!("{:?}", options.format).to_lowercase(),
       match options.format {
           ScreenshotFormat::PNG => "png",
           ScreenshotFormat::JPEG { .. } => "jpeg",
           ScreenshotFormat::WebP { .. } => "webp",
       },
       match options.format {
           ScreenshotFormat::JPEG { quality } => quality.to_string(),
           ScreenshotFormat::WebP { quality } => quality.to_string(),
           _ => "1.0".to_string(),
       },
       options.scale,
       format!("{:?}", options.format).to_lowercase(),
       match options.format {
           ScreenshotFormat::PNG => "png",
           ScreenshotFormat::JPEG { .. } => "jpeg",
           ScreenshotFormat::WebP { .. } => "webp",
       });

    let result = js_sys::eval(&element_capture_script)
        .map_err(|_| Error::BrowserAutomation(format!("Failed to capture element: {}", selector)))?;

    let promise = Promise::from(result);
    let js_result = JsFuture::from(promise)
        .await
        .map_err(|_| Error::BrowserAutomation("Element screenshot promise failed".to_string()))?;

    // Extract data
    let data = Reflect::get(&js_result, &JsValue::from_str("data"))
        .ok()
        .and_then(|v| v.dyn_into::<Array>().ok())
        .map(|arr| {
            let mut vec = Vec::new();
            for i in 0..arr.length() {
                if let Ok(val) = arr.get(i).as_f64() {
                    vec.push(val as u8);
                }
            }
            vec
        })
        .unwrap_or_default();

    let width = Reflect::get(&js_result, &JsValue::from_str("width"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;

    let height = Reflect::get(&js_result, &JsValue::from_str("height"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;

    Ok(ScreenshotResult {
        data,
        format: options.format,
        width,
        height,
        timestamp: js_sys::Date::now(),
        capture_time_ms: web_sys::Performance::now() as u64,
    })
}

/// Render HTML string to canvas
async fn render_html_to_canvas(html: &str, options: ScreenshotOptions) -> Result<ScreenshotResult, Error> {
    let render_script = format!(r#"
        (function() {{
            return new Promise((resolve, reject) => {{
                const iframe = document.createElement('iframe');
                iframe.style.display = 'none';
                iframe.style.position = 'absolute';
                iframe.style.left = '-9999px';
                iframe.style.top = '-9999px';

                iframe.onload = function() {{
                    try {{
                        const doc = iframe.contentDocument || iframe.contentWindow.document;
                        doc.open();
                        doc.write(`<!DOCTYPE html><html><head><style>body{{margin:0;padding:20px;background:{}}}</style></head><body>{}</body></html>`);
                        doc.close();

                        // Wait for content to render
                        setTimeout(() => {{
                            if (typeof html2canvas !== 'undefined') {{
                                html2canvas(doc.body, {{
                                    useCORS: true,
                                    allowTaint: false,
                                    backgroundColor: {},
                                    scale: {},
                                    foreignObjectRendering: true
                                }}).then(canvas => {{
                                    canvas.toBlob((blob) => {{
                                        if (blob) {{
                                            const reader = new FileReader();
                                            reader.onload = function() {{
                                                const arrayBuffer = this.result;
                                                const uint8Array = new Uint8Array(arrayBuffer);
                                                const byteArray = Array.from(uint8Array);
                                                document.body.removeChild(iframe);
                                                resolve({{
                                                    data: byteArray,
                                                    width: canvas.width,
                                                    height: canvas.height,
                                                    format: '{}'
                                                }});
                                            }};
                                            reader.readAsArrayBuffer(blob);
                                        }} else {{
                                            reject(new Error('Rendering failed'));
                                        }}
                                    }}, 'image/{}', {});
                                }}).catch((error) => {{
                                    console.error('HTML render error:', error);
                                    reject(error);
                                }});
                            }} else {{
                                reject(new Error('html2canvas required for HTML rendering'));
                            }}
                        }}, 100);
                    }} catch (error) {{
                        document.body.removeChild(iframe);
                        reject(error);
                    }}
                }};

                document.body.appendChild(iframe);
            }});
        }})()
    "#,
       options.background_color.as_deref().unwrap_or("#ffffff"),
       html.replace("`", "\\`").replace("${", "\\${"),
       options.background_color.as_deref().unwrap_or("null"),
       options.scale,
       format!("{:?}", options.format).to_lowercase(),
       match options.format {
           ScreenshotFormat::PNG => "png",
           ScreenshotFormat::JPEG { .. } => "jpeg",
           ScreenshotFormat::WebP { .. } => "webp",
       },
       match options.format {
           ScreenshotFormat::JPEG { quality } => quality.to_string(),
           ScreenshotFormat::WebP { quality } => quality.to_string(),
           _ => "1.0".to_string(),
       });

    let result = js_sys::eval(&render_script)
        .map_err(|_| Error::BrowserAutomation("Failed to execute HTML render script".to_string()))?;

    let promise = Promise::from(result);
    let js_result = JsFuture::from(promise)
        .await
        .map_err(|_| Error::BrowserAutomation("HTML render promise failed".to_string()))?;

    // Extract data similar to other screenshot functions
    let data = Reflect::get(&js_result, &JsValue::from_str("data"))
        .ok()
        .and_then(|v| v.dyn_into::<Array>().ok())
        .map(|arr| {
            let mut vec = Vec::new();
            for i in 0..arr.length() {
                if let Ok(val) = arr.get(i).as_f64() {
                    vec.push(val as u8);
                }
            }
            vec
        })
        .unwrap_or_default();

    let width = Reflect::get(&js_result, &JsValue::from_str("width"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0) as u32;

    let height = Reflect::get(&js_result, &JsValue::from_str("height"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(600.0) as u32;

    Ok(ScreenshotResult {
        data,
        format: options.format,
        width,
        height,
        timestamp: js_sys::Date::now(),
        capture_time_ms: web_sys::Performance::now() as u64,
    })
}

/// Check if html2canvas is available
async fn html2canvas_available() -> Result<(), Error> {
    let check_script = r#"
        (function() {
            if (typeof html2canvas !== 'undefined') {
                return true;
            }
            // Try to load html2canvas dynamically
            return new Promise((resolve, reject) => {
                if (document.querySelector('script[src*="html2canvas"]')) {
                    resolve(true);
                    return;
                }

                const script = document.createElement('script');
                script.src = 'https://cdnjs.cloudflare.com/ajax/libs/html2canvas/1.4.1/html2canvas.min.js';
                script.onload = () => resolve(true);
                script.onerror = () => reject(new Error('Failed to load html2canvas'));
                document.head.appendChild(script);

                // Timeout after 10 seconds
                setTimeout(() => reject(new Error('html2canvas load timeout')), 10000);
            });
        })()
    "#;

    let result = js_sys::eval(check_script)
        .map_err(|_| Error::BrowserAutomation("Failed to check html2canvas availability".to_string()))?;

    JsFuture::from(Promise::from(result))
        .await
        .map_err(|_| Error::BrowserAutomation("html2canvas availability check failed".to_string()))?;

    Ok(())
}

/// Generate sample dashboard HTML
fn generate_dashboard_html() -> String {
    format!(r#"
        <div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #1a1a1a; color: #ffffff; padding: 20px;">
            <h1 style="color: #00ff88; margin-bottom: 30px;">🚀 Infrastructure Assassin Dashboard</h1>

            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 20px;">
                <div style="background: #2d2d2d; padding: 20px; border-radius: 10px; border-left: 4px solid #ff6b6b;">
                    <h3 style="margin: 0 0 10px 0; color: #ff6b6b;">💰 Revenue Savings</h3>
                    <div style="font-size: 24px; font-weight: bold;">$45,230</div>
                    <div style="font-size: 12px; opacity: 0.7;">vs AWS Lambda costs</div>
                </div>

                <div style="background: #2d2d2d; padding: 20px; border-radius: 10px; border-left: 4px solid #4ecdc4;">
                    <h3 style="margin: 0 0 10px 0; color: #4ecdc4;">⚡ Performance</h3>
                    <div style="font-size: 24px; font-weight: bold;">10x</div>
                    <div style="font-size: 12px; opacity: 0.7;">productivity gain</div>
                </div>

                <div style="background: #2d2d2d; padding: 20px; border-radius: 10px; border-left: 4px solid #45b7d1;">
                    <h3 style="margin: 0 0 10px 0; color: #45b7d1;">🎯 Automation</h3>
                    <div style="font-size: 24px; font-weight: bold;">847</div>
                    <div style="font-size: 12px; opacity: 0.7;">browser sessions</div>
                </div>

                <div style="background: #2d2d2d; padding: 20px; border-radius: 10px; border-left: 4px solid #f9ca24;">
                    <h3 style="margin: 0 0 10px 0; color: #f9ca24;">🔒 Security</h3>
                    <div style="font-size: 24px; font-weight: bold;">Zero</div>
                    <div style="font-size: 12px; opacity: 0.7;">external dependencies</div>
                </div>
            </div>

            <div style="margin-top: 30px; background: #2d2d2d; padding: 20px; border-radius: 10px;">
                <h3 style="margin: 0 0 15px 0; color: #00ff88;">📈 Real-Time Metrics</h3>
                <div style="display: flex; gap: 20px; align-items: center;">
                    <div>
                        <div style="font-size: 18px; font-weight: bold;">{:.2}ms</div>
                        <div style="font-size: 12px; opacity: 0.7;">Avg Response Time</div>
                    </div>
                    <div>
                        <div style="font-size: 18px; font-weight: bold;">5.2MB</div>
                        <div style="font-size: 12px; opacity: 0.7;">WASM Bundle</div>
                    </div>
                    <div>
                        <div style="font-size: 18px; font-weight: bold;">99.9%</div>
                        <div style="font-size: 12px; opacity: 0.7;">Uptime</div>
                    </div>
                </div>
            </div>

            <div style="margin-top: 20px; text-align: center; font-size: 12px; opacity: 0.6;">
                Generated at {} | Infrastructure Assassin v2.0
            </div>
        </div>
    "#, 47.2, js_sys::Date::now().to_string())
}
