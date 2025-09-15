//! Browser storage integration capabilities
//!
//! This module provides comprehensive browser storage management including
//! localStorage, sessionStorage, IndexedDB, and cache API integration.

use crate::Error;
use futures::Stream;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, window, Storage, IdbDatabase, IdbObjectStore, IdbTransaction, IdbKeyRange, Cache, Request, Response};
use js_sys::{Array, Object, Promise, Reflect, Date};
use serde::{Deserialize, Serialize};

/// Storage types available in browsers
#[derive(Debug, Clone)]
pub enum StorageType {
    LocalStorage,
    SessionStorage,
    IndexedDB { database: String, store: String },
    CacheAPI { cache_name: String },
}

/// Storage operation result
#[derive(Debug, Clone)]
pub struct StorageResult {
    pub success: bool,
    pub key: String,
    pub value: Option<String>,
    pub timestamp: f64,
    pub storage_type: StorageType,
}

/// Session state data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub agent_states: std::collections::HashMap<String, AgentState>,
    pub user_context: UserContext,
    pub timestamp: f64,
    pub version: String,
}

/// Agent state within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: String,
    pub capabilities: Vec<String>,
    pub last_action: String,
    pub performance_metrics: PerformanceMetrics,
}

/// User context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: Option<String>,
    pub preferences: std::collections::HashMap<String, String>,
    pub device_info: DeviceInfo,
}

/// Device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub user_agent: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub pixel_ratio: f64,
    pub language: String,
    pub timezone: String,
}

/// Performance metrics for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub execution_time: f64,
    pub memory_usage: usize,
    pub success_rate: f64,
    pub interaction_count: u64,
}

/// Storage policy for cache expiration and cleanup
#[derive(Debug, Clone)]
pub struct StoragePolicy {
    pub max_age_seconds: u64,
    pub max_items: usize,
    pub compression_enabled: bool,
    pub auto_cleanup: bool,
}

/// Store session state in persistent storage
pub async fn store_session_state(key: &str, state: SessionState) -> Result<(), Error> {
    // Try IndexedDB first (most reliable), fallback to localStorage
    if let Ok(_) = store_in_indexeddb("infrastructure_assassin_sessions", "states", key, &state).await {
        log::info!("Session state stored in IndexedDB: {}", key);
        return Ok(());
    }

    // Fallback to localStorage with JSON serialization
    let state_json = serde_json::to_string(&state)
        .map_err(|_| Error::BrowserAutomation("Failed to serialize session state".to_string()))?;

    store_in_localstorage(key, &state_json).await?;
    log::info!("Session state stored in localStorage: {}", key);
    Ok(())
}

/// Retrieve cached session state
pub async fn retrieve_cached_session(key: &str) -> Result<Option<SessionState>, Error> {
    // Try IndexedDB first
    if let Ok(Some(state)) = retrieve_from_indexeddb::<SessionState>("infrastructure_assassin_sessions", "states", key).await {
        log::debug!("Session state retrieved from IndexedDB: {}", key);
        return Ok(Some(state));
    }

    // Fallback to localStorage
    if let Ok(Some(json)) = retrieve_from_localstorage(key).await {
        let state: SessionState = serde_json::from_str(&json)
            .map_err(|_| Error::BrowserAutomation("Failed to deserialize session state".to_string()))?;
        log::debug!("Session state retrieved from localStorage: {}", key);
        return Ok(Some(state));
    }

    log::debug!("Session state not found: {}", key);
    Ok(None)
}

/// Store module data in cache with efficient retrieval
pub async fn store_cached_module(module_name: &str, module_data: &[u8]) -> Result<(), Error> {
    // Store module metadata in IndexedDB
    let metadata = ModuleMetadata {
        name: module_name.to_string(),
        size: module_data.len(),
        checksum: calculate_checksum(module_data),
        timestamp: Date::now(),
        version: "1.0.0".to_string(),
    };

    store_in_indexeddb("infrastructure_assassin_modules", "metadata", module_name, &metadata).await?;

    // Store actual data in Cache API for efficiency
    store_in_cache_api(&format!("ia_module_{}", module_name), module_data).await?;

    log::info!("Module cached: {} ({} bytes)", module_name, module_data.len());
    Ok(())
}

/// Retrieve cached module data
pub async fn retrieve_cached_module(module_name: &str) -> Result<Option<Vec<u8>>, Error> {
    // Check cache freshness
    if let Ok(Some(metadata)) = retrieve_from_indexeddb::<ModuleMetadata>("infrastructure_assassin_modules", "metadata", module_name).await {
        let age_seconds = (Date::now() - metadata.timestamp) / 1000.0;

        // Check if module is still fresh (cache for 7 days)
        if age_seconds < 604800.0 {
            let data = retrieve_from_cache_api(&format!("ia_module_{}", module_name)).await?;
            log::debug!("Module retrieved from cache: {} ({:.1}s old)", module_name, age_seconds);
            return Ok(data);
        }

        // Module is stale, clean it up
        cleanup_stale_module(module_name).await.ok();
    }

    Ok(None)
}

/// Sync with IndexedDB for reliable persistence
pub async fn sync_with_indexed_db() -> Result<(), Error> {
    let sync_data = SyncData {
        last_sync: Date::now(),
        version: "2.0.0".to_string(),
        capabilities: vec![
            "session_states".to_string(),
            "module_cache".to_string(),
            "network_cache".to_string(),
            "analytics_data".to_string(),
        ],
    };

    store_in_indexeddb("infrastructure_assassin_sync", "status", "primary", &sync_data).await?;
    log::info!("Storage sync completed with IndexedDB");
    Ok(())
}

/// Store data in localStorage
async fn store_in_localstorage(key: &str, value: &str) -> Result<(), Error> {
    let window = window().ok_or_else(|| Error::BrowserAutomation("No global window available".to_string()))?;
    let storage = window.local_storage()
        .map_err(|_| Error::BrowserAutomation("Failed to access localStorage".to_string()))?
        .ok_or_else(|| Error::BrowserAutomation("localStorage not available".to_string()))?;

    storage.set_item(&format!("ia_{}", key), value)
        .map_err(|_| Error::BrowserAutomation("Failed to store in localStorage".to_string()))?;

    Ok(())
}

/// Retrieve data from localStorage
async fn retrieve_from_localstorage(key: &str) -> Result<Option<String>, Error> {
    let window = window().ok_or_else(|| Error::BrowserAutomation("No global window available".to_string()))?;
    let storage = window.local_storage()
        .map_err(|_| Error::BrowserAutomation("Failed to access localStorage".to_string()))?
        .ok_or_else(|| Error::BrowserAutomation("localStorage not available".to_string()))?;

    let value = storage.get_item(&format!("ia_{}", key))
        .map_err(|_| Error::BrowserAutomation("Failed to retrieve from localStorage".to_string()))?;

    Ok(value)
}

/// Store data in IndexedDB
async fn store_in_indexeddb<T: serde::Serialize>(database: &str, store: &str, key: &str, value: &T) -> Result<(), Error> {
    let store_script = format!(r#"
        (function() {{
            const data = {};
            const dbName = '{}';
            const storeName = '{}';
            const itemKey = '{}';

            return new Promise((resolve, reject) => {{
                const request = indexedDB.open(dbName, 1);

                request.onupgradeneeded = function(event) {{
                    const db = event.target.result;
                    if (!db.objectStoreNames.contains(storeName)) {{
                        db.createObjectStore(storeName);
                    }}
                }};

                request.onsuccess = function(event) {{
                    const db = event.target.result;
                    const transaction = db.transaction([storeName], 'readwrite');
                    const store = transaction.objectStore(storeName);

                    const putRequest = store.put(data, itemKey);

                    putRequest.onsuccess = function() {{
                        db.close();
                        resolve(true);
                    }};

                    putRequest.onerror = function() {{
                        db.close();
                        reject(new Error('Failed to store data'));
                    }};
                }};

                request.onerror = function() {{
                    reject(new Error('Failed to open database'));
                }};
            }});
        }})()
    "#, serde_json::to_string(value).unwrap_or_default(), database, store, key);

    let result = js_sys::eval(&store_script)
        .map_err(|_| Error::BrowserAutomation("Failed to execute IndexedDB store".to_string()))?;

    JsFuture::from(Promise::from(result))
        .await
        .map_err(|_| Error::BrowserAutomation("IndexedDB store promise failed".to_string()))?;

    Ok(())
}

/// Retrieve data from IndexedDB
async fn retrieve_from_indexeddb<T: serde::DeserializeOwned>(database: &str, store: &str, key: &str) -> Result<Option<T>, Error> {
    let retrieve_script = format!(r#"
        (function() {{
            const dbName = '{}';
            const storeName = '{}';
            const itemKey = '{}';

            return new Promise((resolve, reject) => {{
                const request = indexedDB.open(dbName, 1);

                request.onupgradeneeded = function(event) {{
                    const db = event.target.result;
                    if (!db.objectStoreNames.contains(storeName)) {{
                        db.createObjectStore(storeName);
                    }}
                }};

                request.onsuccess = function(event) {{
                    const db = event.target.result;
                    const transaction = db.transaction([storeName], 'readonly');
                    const objectStore = transaction.objectStore(storeName);

                    const getRequest = objectStore.get(itemKey);

                    getRequest.onsuccess = function(event) {{
                        const result = event.target.result;
                        db.close();
                        if (result !== undefined) {{
                            resolve(JSON.stringify(result));
                        }} else {{
                            resolve(null);
                        }}
                    }};

                    getRequest.onerror = function() {{
                        db.close();
                        reject(new Error('Failed to retrieve data'));
                    }};
                }};

                request.onerror = function() {{
                    reject(new Error('Failed to open database'));
                }};
            }});
        }})()
    "#, database, store, key);

    let result = js_sys::eval(&retrieve_script)
        .map_err(|_| Error::BrowserAutomation("Failed to execute IndexedDB retrieve".to_string()))?;

    let json_result = JsFuture::from(Promise::from(result))
        .await
        .map_err(|_| Error::BrowserAutomation("IndexedDB retrieve promise failed".to_string()))?;

    if let Some(json_str) = json_result.as_string() {
        if json_str == "null" || json_str.is_empty() {
            return Ok(None);
        }

        let value: T = serde_json::from_str(&json_str)
            .map_err(|_| Error::BrowserAutomation("Failed to deserialize from IndexedDB".to_string()))?;

        Ok(Some(value))
    } else {
        Ok(None)
    }
}

/// Store data in Cache API
async fn store_in_cache_api(cache_name: &str, data: &[u8]) -> Result<(), Error> {
    let cache_script = format!(r#"
        (function() {{
            const cacheName = '{}';
            const data = new Uint8Array({});

            return caches.open(cacheName).then(cache => {{
                const response = new Response(data, {{
                    headers: {{ 'content-type': 'application/octet-stream' }}
                }});
                return cache.put('data', response);
            }});
        }})()
    "#, cache_name, format!("{:?}", data).replace("[", "[").replace("]", "]"));

    let result = js_sys::eval(&cache_script)
        .map_err(|_| Error::BrowserAutomation("Failed to execute Cache API store".to_string()))?;

    JsFuture::from(Promise::from(result))
        .await
        .map_err(|_| Error::BrowserAutomation("Cache API store promise failed".to_string()))?;

    Ok(())
}

/// Retrieve data from Cache API
async fn retrieve_from_cache_api(cache_name: &str) -> Result<Option<Vec<u8>>, Error> {
    let retrieve_script = format!(r#"
        (function() {{
            const cacheName = '{}';

            return caches.open(cacheName).then(cache => {{
                return cache.match('data');
            }}).then(response => {{
                if (response) {{
                    return response.arrayBuffer();
                }}
                return null;
            }});
        }})()
    "#, cache_name);

    let result = js_sys::eval(&retrieve_script)
        .map_err(|_| Error::BrowserAutomation("Failed to execute Cache API retrieve".to_string()))?;

    let response = JsFuture::from(Promise::from(result))
        .await
        .map_err(|_| Error::BrowserAutomation("Cache API retrieve promise failed".to_string()))?;

    if response.is_null() {
        return Ok(None);
    }

    // Convert ArrayBuffer to Vec<u8>
    if let Ok(array_buffer) = response.dyn_into::<js_sys::ArrayBuffer>() {
        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        let mut data = vec![0; uint8_array.length() as usize];
        uint8_array.copy_to(&mut data);
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

/// Calculate checksum for data integrity
fn calculate_checksum(data: &[u8]) -> String {
    // Simple checksum implementation
    let mut checksum = 0u32;
    for &byte in data {
        checksum = checksum.wrapping_add(byte as u32);
    }
    format!("{:08x}", checksum)
}

/// Module metadata for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModuleMetadata {
    name: String,
    size: usize,
    checksum: String,
    timestamp: f64,
    version: String,
}

/// Sync data for storage coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncData {
    last_sync: f64,
    version: String,
    capabilities: Vec<String>,
}

/// Clean up stale module cache
async fn cleanup_stale_module(module_name: &str) -> Result<(), Error> {
    // Remove from Cache API
    let cache_script = format!(r#"
        (function() {{
            const cacheName = 'ia_module_{}';
            return caches.delete(cacheName);
        }})()
    "#, module_name);

    let _ = js_sys::eval(&cache_script)
        .map_err(|_| Error::BrowserAutomation("Failed to cleanup stale cache".to_string()))?;

    // Remove metadata from IndexedDB
    let delete_script = format!(r#"
        (function() {{
            return new Promise((resolve, reject) => {{
                const request = indexedDB.open('infrastructure_assassin_modules', 1);

                request.onsuccess = function(event) {{
                    const db = event.target.result;
                    const transaction = db.transaction(['metadata'], 'readwrite');
                    const store = transaction.objectStore('metadata');

                    const deleteRequest = store.delete('{}');

                    deleteRequest.onsuccess = function() {{
                        db.close();
                        resolve(true);
                    }};

                    deleteRequest.onerror = function() {{
                        db.close();
                        reject(new Error('Failed to delete metadata'));
                    }};
                }};

                request.onerror = function() {{
                    reject(new Error('Failed to open database'));
                }};
            }});
        }})()
    "#, module_name);

    let _ = js_sys::eval(&delete_script)
        .map_err(|_| Error::BrowserAutomation("Failed to cleanup metadata".to_string()))?;

    log::debug!("Cleaned up stale module: {}", module_name);
    Ok(())
}

/// Get storage usage statistics
pub async fn get_storage_stats() -> Result<StorageStats, Error> {
    let stats_script = r#"
        (function() {
            return navigator.storage.estimate().then(estimate => {
                return {
                    localStorage: localStorage.length,
                    sessionStorage: sessionStorage ? sessionStorage.length : 0,
                    indexedDB: estimate.usage || 0,
                    cache: estimate.quota || 0,
                    available: estimate.quota - estimate.usage || 0
                };
            });
        })()
    "#;

    let result = js_sys::eval(stats_script)
        .map_err(|_| Error::BrowserAutomation("Failed to get storage stats".to_string()))?;

    let stats = JsFuture::from(Promise::from(result))
        .await
        .map_err(|_| Error::BrowserAutomation("Storage stats promise failed".to_string()))?;

    let local_storage = Reflect::get(&stats, &JsValue::from_str("localStorage"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u64;

    let session_storage = Reflect::get(&stats, &JsValue::from_str("sessionStorage"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u64;

    let indexed_db = Reflect::get(&stats, &JsValue::from_str("indexedDB"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u64;

    let cache_quota = Reflect::get(&stats, &JsValue::from_str("cache"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u64;

    let available = Reflect::get(&stats, &JsValue::from_str("available"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u64;

    Ok(StorageStats {
        local_storage_items: local_storage as usize,
        session_storage_items: session_storage as usize,
        indexed_db_usage: indexed_db,
        cache_quota,
        available_space: available,
    })
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub local_storage_items: usize,
    pub session_storage_items: usize,
    pub indexed_db_usage: u64,
    pub cache_quota: u64,
    pub available_space: u64,
}
