#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const SLOT_COUNT: usize = 20;
const MAX_ACTIVE_OVERLAYS: usize = runtime_limits::MAX_ACTIVE_INSTANCES;
const SERVER_ADDR: &str = "127.0.0.1:8790";

#[derive(Clone, Serialize, Deserialize)]
struct Slot {
    name: String,
    html_file: Option<String>,
    active: bool,
    pinned: bool,
    duration_secs: u64,
    layer: i32,
}
impl Slot {
    fn new(index: usize) -> Self {
        Self {
            name: match index {
                0 => "Black Cat".into(),
                1 => "Halloween Sides".into(),
                2 => "Flying Witch".into(),
                3 => "Warlock".into(),
                4 => "Pumpkin".into(),
                5 => "Ghost".into(),
                _ => format!("Overlay {:02}", index + 1),
            },
            html_file: None,
            active: false,
            pinned: false,
            duration_secs: 15,
            layer: 10 + index as i32,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct DeckConfig {
    slots: Vec<Slot>,
}
impl Default for DeckConfig {
    fn default() -> Self {
        Self { slots: (0..SLOT_COUNT).map(Slot::new).collect() }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ActiveOverlayInstance {
    instance_id: String,
    deck_id: String,
    slot: usize,
    name: String,
    file: String,
    url: String,
    layer: i32,
    activated_at_unix: u64,
}

struct Shared {
    config: DeckConfig,
    root: PathBuf,
    message: String,
    server_status: String,
    revision: u64,
    expirations: Vec<Option<Instant>>,
    memory: memory_monitor::MemoryStatus,
    current_deck_id: Option<String>,
    active_instances: Vec<ActiveOverlayInstance>,
    next_instance_id: u64,
    active_registry: cache_manager::ActiveOverlayRegistry,
}
impl Shared {
    fn config_path(&self) -> PathBuf { self.root.join("deck-config.json") }
    fn overlays_root(&self) -> PathBuf { self.root.join("Overlays") }
    fn save_config(&mut self) {
        let _ = fs::create_dir_all(&self.root);
        if let Ok(data) = serde_json::to_vec_pretty(&self.config) {
            if fs::write(self.config_path(), data).is_ok() {
                self.message = "Deck configuration saved.".into();
            } else {
                self.message = "Could not save deck-config.json.".into();
            }
        }
    }
    fn touch(&mut self) { self.revision = self.revision.wrapping_add(1); }
}

fn app_root() -> PathBuf {
    // Keep editable overlay content next to the app executable, not in a temp directory.
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn load_config(root: &Path) -> DeckConfig {
    let path = root.join("deck-config.json");
    let mut config = if let Ok(data) = fs::read(path) {
        serde_json::from_slice::<DeckConfig>(&data)
            .ok()
            .filter(|c| c.slots.len() == SLOT_COUNT)
            .unwrap_or_default()
    } else {
        DeckConfig::default()
    };
    // Never resume an overlay automatically after app restart.
    for slot in &mut config.slots {
        slot.active = false;
        slot.pinned = false;
    }
    config
}

fn safe_slot_path(root: &Path, slot: usize, rel: &str) -> Option<PathBuf> {
    if slot >= SLOT_COUNT { return None; }
    let rel_path = Path::new(rel);
    if rel_path.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
        return None;
    }
    Some(root.join("Overlays").join(format!("slot-{:02}", slot + 1)).join(rel_path))
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn write_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: http://127.0.0.1:8790\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
}

fn read_request(stream: &mut TcpStream) -> Option<(String, String, Vec<u8>)> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end;
    loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 { return None; }
        bytes.extend_from_slice(&chunk[..n]);
        if bytes.len() > 25 * 1024 * 1024 { return None; }
        if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if bytes.len() > 64 * 1024 { return None; }
    }
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let mut lines = headers.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let target = first.next()?.to_string();
    let content_length = lines.find_map(|line| {
        let (k, v) = line.split_once(':')?;
        if k.eq_ignore_ascii_case("content-length") { v.trim().parse::<usize>().ok() } else { None }
    }).unwrap_or(0).min(20 * 1024 * 1024);
    while bytes.len().saturating_sub(header_end) < content_length {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 { break; }
        bytes.extend_from_slice(&chunk[..n]);
    }
    let body_end = (header_end + content_length).min(bytes.len());
    Some((method, target, bytes[header_end..body_end].to_vec()))
}

fn decode_query_value(value: &str) -> String {
    let mut out = Vec::new();
    let b = value.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&value[i+1..i+3], 16) { out.push(v); i += 3; continue; }
        }
        out.push(if b[i] == b'+' { b' ' } else { b[i] }); i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn safe_relative_path(value: &str) -> Option<PathBuf> {
    let p = Path::new(value);
    if value.is_empty() || p.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_))) { return None; }
    Some(p.to_path_buf())
}

fn core_packs_root(root: &Path) -> PathBuf {
    // Installed builds keep Core_Packs beside StimTake.exe. The portable release
    // archive also contains the executable one level above the source project,
    // so support that layout without requiring users to import built-in overlays.
    let direct = root.join("Core_Packs");
    if direct.is_dir() { return direct; }
    let nested = root.join("StimTake-Rust-UI-Concept-Build").join("Core_Packs");
    if nested.is_dir() { return nested; }
    direct
}

fn find_pack_file(root: &Path, filename: &str) -> Option<PathBuf> {
    fn walk(dir: &Path, filename: &str, depth: usize) -> Option<PathBuf> {
        if depth > 8 { return None; }
        for entry in fs::read_dir(dir).ok()? {
            let entry = entry.ok()?; let path = entry.path();
            if path.is_dir() { if let Some(found) = walk(&path, filename, depth + 1) { return Some(found); } }
            else if path.file_name().and_then(|n| n.to_str()).map(|n| n.eq_ignore_ascii_case(filename)).unwrap_or(false) { return Some(path); }
        }
        None
    }
    walk(&core_packs_root(root), filename, 0)
}

fn url_for_slot(root: &Path, slot_index: usize, filename: &str) -> Option<String> {
    let rel = safe_relative_path(filename)?;
    let local = root.join("Overlays").join(format!("slot-{:02}", slot_index + 1)).join(&rel);
    if local.is_file() {
        let encoded = rel.components().filter_map(|c| c.as_os_str().to_str()).map(|seg| seg.replace('%', "%25").replace(' ', "%20").replace('#', "%23").replace('?', "%3F")).collect::<Vec<_>>().join("/");
        return Some(format!("/overlay/{:02}/{}", slot_index + 1, encoded));
    }
    let pack_root = core_packs_root(root);
    // Manifest-backed built-in decks use an explicit folder/file path so two packs
    // can safely contain similarly named overlays without resolving to the wrong file.
    let found = if rel.components().count() > 1 {
        let candidate = pack_root.join(&rel);
        if candidate.is_file() { Some(candidate) } else { find_pack_file(root, rel.file_name()?.to_str()?) }
    } else {
        find_pack_file(root, rel.file_name()?.to_str()?)
    }?;
    let pack_rel = found.strip_prefix(&pack_root).ok()?;
    let encoded = pack_rel.components().filter_map(|c| c.as_os_str().to_str()).map(|seg| seg.replace('%', "%25").replace(' ', "%20").replace('#', "%23").replace('?', "%3F")).collect::<Vec<_>>().join("/");
    Some(format!("/packs/{}", encoded))
}

fn load_pack_catalog(root: &Path) -> Vec<serde_json::Value> {
    let packs_root = core_packs_root(root);
    let mut packs: Vec<serde_json::Value> = Vec::new();
    let Ok(entries) = fs::read_dir(&packs_root) else { return packs; };
    for entry in entries.flatten() {
        let folder = entry.path();
        if !folder.is_dir() { continue; }
        let manifest_path = folder.join("deck.json");
        let Ok(bytes) = fs::read(&manifest_path) else { continue; };
        let Ok(mut manifest) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue; };
        let Some(slots) = manifest.get("slots").and_then(|v| v.as_array()) else { continue; };
        if slots.len() != SLOT_COUNT { continue; }
        let mut valid = true;
        for slot in slots {
            let file = slot.get("file").and_then(|v| v.as_str()).unwrap_or("");
            if file.is_empty() || safe_relative_path(file).is_none() || !folder.join(file).is_file() {
                valid = false; break;
            }
        }
        if !valid { continue; }
        let folder_name = folder.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if let Some(obj) = manifest.as_object_mut() { obj.insert("folder".into(), serde_json::Value::String(folder_name)); }
        packs.push(manifest);
    }
    packs.sort_by(|a,b| a.get("name").and_then(|v|v.as_str()).unwrap_or("").cmp(b.get("name").and_then(|v|v.as_str()).unwrap_or("")));
    packs
}

fn handle_http(mut stream: TcpStream, shared: &Arc<Mutex<Shared>>) {
    let (method, target, body) = match read_request(&mut stream) { Some(v) => v, None => return };
    let path = target.split('?').next().unwrap_or("/");
    if method == "OPTIONS" {
        let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: http://127.0.0.1:8790\r\nAccess-Control-Allow-Methods: GET, HEAD, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"); return;
    }
    if method == "POST" && path == "/api/slots" {
        let parsed: serde_json::Value = match serde_json::from_slice(&body) { Ok(v) => v, Err(_) => { write_response(&mut stream, "400 Bad Request", "application/json", br#"{"error":"Invalid JSON"}"#); return; } };
        let slots = match parsed.get("slots").and_then(|v| v.as_array()) { Some(v) if v.len() == SLOT_COUNT => v, _ => { write_response(&mut stream, "400 Bad Request", "application/json", br#"{"error":"Expected exactly 20 slots"}"#); return; } };
        let incoming_deck_id = parsed.get("deckId").and_then(|v| v.as_str()).unwrap_or("default-deck").chars().take(160).collect::<String>();
        let mut s = shared.lock().unwrap();
        let deck_changed = s.current_deck_id.as_deref()
            .map(|current| current != incoming_deck_id.as_str())
            .unwrap_or(false);

        // A deck switch is a hard session boundary: stop every live overlay from
        // the outgoing deck before loading the incoming deck, unregister protected
        // resources, clear the compositor's instance list, and start the new deck
        // with every overlay OFF. Never inherit a running state across decks.
        if deck_changed {
            let stopped_instances = std::mem::take(&mut s.active_instances);
            for instance in stopped_instances {
                s.active_registry.unregister_instance(&instance.instance_id);
            }
            for slot in &mut s.config.slots {
                slot.active = false;
            }
            s.expirations.fill(None);
        }

        // Preflight the entire update before activating anything. During a deck
        // switch the incoming deck is always initialized inactive; cards still
        // load from its manifest and can be activated by the user afterward.
        let requested_active_file = |slot_number: usize| -> Option<String> {
            if deck_changed { return None; }
            slots.get(slot_number.saturating_sub(1)).and_then(|value| {
                if value.get("active").and_then(|v| v.as_bool()).unwrap_or(false) {
                    value.get("file").and_then(|v| v.as_str()).map(str::trim)
                        .filter(|v| !v.is_empty()).and_then(|v| safe_relative_path(v))
                        .map(|p| p.to_string_lossy().to_string())
                } else { None }
            })
        };
        let retained_count = s.active_instances.iter().filter(|instance| {
            if instance.deck_id != incoming_deck_id { return true; }
            requested_active_file(instance.slot).as_deref() == Some(instance.file.as_str())
        }).count();
        let new_activation_count = (1..=SLOT_COUNT).filter(|slot_number| {
            let Some(file) = requested_active_file(*slot_number) else { return false; };
            !s.active_instances.iter().any(|instance| instance.deck_id == incoming_deck_id
                && instance.slot == *slot_number && instance.file == file)
        }).count();
        if let Err(denial) = runtime_limits::can_activate(retained_count, new_activation_count, s.memory.pressure, MAX_ACTIVE_OVERLAYS) {
            let body = serde_json::to_vec(&serde_json::json!({
                "ok": false, "error": denial.message(), "code": denial.code(),
                "activeCount": s.active_instances.len(), "maxActive": MAX_ACTIVE_OVERLAYS,
                "pressure": s.memory.pressure
            })).unwrap_or_else(|_| b"{\"ok\":false,\"error\":\"Activation denied\"}".to_vec());
            write_response(&mut stream, "409 Conflict", "application/json", &body); return;
        }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        for (i, value) in slots.iter().enumerate() {
            let name = value.get("name").and_then(|v| v.as_str()).unwrap_or("Overlay").chars().take(80).collect::<String>();
            let file_raw = value.get("file").and_then(|v| v.as_str()).unwrap_or("").trim();
            let file = if file_raw.is_empty() { None } else { safe_relative_path(file_raw).map(|p| p.to_string_lossy().to_string()) };
            let active = !deck_changed && value.get("active").and_then(|v| v.as_bool()).unwrap_or(false) && file.is_some();
            let layer = value.get("layer").and_then(|v| v.as_i64()).unwrap_or(10 + i as i64) as i32;

            // After a deck switch all previous instances were removed above;
            // this loop only reconciles the incoming deck's inactive card state.
            let matching = s.active_instances.iter().position(|instance| instance.deck_id == incoming_deck_id && instance.slot == i + 1);
            if let Some(index) = matching {
                let same_file = file.as_deref() == Some(s.active_instances[index].file.as_str());
                if !active || !same_file {
                    let removed = s.active_instances.remove(index);
                    s.active_registry.unregister_instance(&removed.instance_id);
                } else {
                    s.active_instances[index].name = name.clone();
                    s.active_instances[index].layer = layer;
                }
            }
            if active && !s.active_instances.iter().any(|instance| instance.deck_id == incoming_deck_id && instance.slot == i + 1 && Some(instance.file.as_str()) == file.as_deref()) {
                if let Some(file_value) = file.as_deref() {
                    if let Some(url) = url_for_slot(&s.root, i, file_value) {
                        s.next_instance_id = s.next_instance_id.wrapping_add(1).max(1);
                        let instance_id = format!("overlay-{}-{}", i + 1, s.next_instance_id);
                        s.active_registry.register_instance(instance_id.clone(), [url.clone()]);
                        s.active_instances.push(ActiveOverlayInstance { instance_id, deck_id: incoming_deck_id.clone(), slot: i + 1, name: name.clone(), file: file_value.to_string(), url, layer, activated_at_unix: now });
                    }
                }
            }

            let slot = &mut s.config.slots[i];
            slot.name = name;
            slot.html_file = file;
            slot.active = active;
            slot.layer = layer;
            if let Some(duration) = value.get("duration_secs").and_then(|v| v.as_u64()) { slot.duration_secs = duration.min(86400); }
        }
        s.current_deck_id = Some(incoming_deck_id);
        s.touch(); s.save_config();
        let body = serde_json::to_vec(&serde_json::json!({
            "ok": true, "activeCount": s.active_instances.len(),
            "maxActive": MAX_ACTIVE_OVERLAYS, "pressure": s.memory.pressure
        })).unwrap_or_else(|_| br#"{"ok":true}"#.to_vec());
        write_response(&mut stream, "200 OK", "application/json", &body); return;
    }
    if method == "DELETE" && path.starts_with("/api/instances/") {
        let instance_id = path.trim_start_matches("/api/instances/");
        if instance_id.is_empty() || !instance_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            write_response(&mut stream, "400 Bad Request", "application/json", br#"{"error":"Invalid instance id"}"#); return;
        }
        let mut s = shared.lock().unwrap();
        if let Some(index) = s.active_instances.iter().position(|item| item.instance_id == instance_id) {
            let removed = s.active_instances.remove(index);
            s.active_registry.unregister_instance(&removed.instance_id);
            // Only reflect the stop into the current slot when it is the same deck.
            if s.current_deck_id.as_deref() == Some(removed.deck_id.as_str()) && (1..=SLOT_COUNT).contains(&removed.slot) {
                s.config.slots[removed.slot - 1].active = false;
            }
            s.touch(); s.save_config();
            write_response(&mut stream, "200 OK", "application/json", br#"{"ok":true}"#);
        } else {
            write_response(&mut stream, "404 Not Found", "application/json", br#"{"error":"Active instance not found"}"#);
        }
        return;
    }
    if method == "POST" && path.starts_with("/api/import/") {
        let slot_num = path.trim_start_matches("/api/import/").parse::<usize>().ok();
        let slot_index = match slot_num { Some(n) if (1..=SLOT_COUNT).contains(&n) => n - 1, _ => { write_response(&mut stream, "400 Bad Request", "application/json", br#"{"error":"Invalid slot"}"#); return; } };
        let name = target.split('?').nth(1).unwrap_or("").split('&').find_map(|p| p.strip_prefix("name=")).map(decode_query_value).unwrap_or_else(|| "overlay.html".into());
        let rel = match safe_relative_path(&name) { Some(p) if p.extension().and_then(|e| e.to_str()).map(|e| ["html", "htm", "css", "js", "json", "png", "jpg", "jpeg", "gif", "svg", "webp", "mp3", "mp4"].iter().any(|x| x.eq_ignore_ascii_case(e))).unwrap_or(false) => p, _ => { write_response(&mut stream, "400 Bad Request", "application/json", br#"{"error":"Invalid or unsupported filename"}"#); return; } };
        if body.len() > 20 * 1024 * 1024 { write_response(&mut stream, "413 Payload Too Large", "application/json", br#"{"error":"File too large"}"#); return; }
        let dest = { let s = shared.lock().unwrap(); s.root.join("Overlays").join(format!("slot-{:02}", slot_index + 1)).join(&rel) };
        if let Some(parent) = dest.parent() { if fs::create_dir_all(parent).is_err() { write_response(&mut stream, "500 Internal Server Error", "application/json", br#"{"error":"Could not create overlay folder"}"#); return; } }
        if fs::write(&dest, &body).is_err() { write_response(&mut stream, "500 Internal Server Error", "application/json", br#"{"error":"Could not save overlay"}"#); return; }
        write_response(&mut stream, "200 OK", "application/json", br#"{"ok":true}"#); return;
    }
    if method == "DELETE" && path.starts_with("/api/import/") {
        let slot_num = path.trim_start_matches("/api/import/").parse::<usize>().ok();
        let slot_index = match slot_num { Some(n) if (1..=SLOT_COUNT).contains(&n) => n - 1, _ => { write_response(&mut stream, "400 Bad Request", "application/json", br#"{"error":"Invalid slot"}"#); return; } };
        let name = target.split('?').nth(1).unwrap_or("").split('&').find_map(|p| p.strip_prefix("name=")).map(decode_query_value).unwrap_or_default();
        let rel = match safe_relative_path(&name) { Some(p) => p, None => { write_response(&mut stream, "400 Bad Request", "application/json", br#"{"error":"Invalid filename"}"#); return; } };
        let dest = { let s = shared.lock().unwrap(); s.root.join("Overlays").join(format!("slot-{:02}", slot_index + 1)).join(rel) };
        match fs::remove_file(&dest) { Ok(()) => write_response(&mut stream, "200 OK", "application/json", br#"{"ok":true}"#), Err(_) => write_response(&mut stream, "404 Not Found", "application/json", br#"{"error":"File not found"}"#) }; return;
    }
    if method != "GET" && method != "HEAD" {
        write_response(&mut stream, "405 Method Not Allowed", "text/plain", b"Method not allowed"); return;
    }
    if path == "/" || path == "/index.html" { write_response(&mut stream, "200 OK", "text/html; charset=utf-8", UI_HTML.as_bytes()); return; }
    if path == "/health" { write_response(&mut stream, "200 OK", "text/plain; charset=utf-8", b"StimTake is running."); return; }
    if path == "/api/memory" { let status = { shared.lock().map(|s| s.memory.clone()).unwrap_or_default() }; let data = serde_json::to_vec(&status).unwrap_or_else(|_| b"{}".to_vec()); write_response(&mut stream, "200 OK", "application/json; charset=utf-8", &data); return; }
    if path == "/api/packs" { let root = { let s = shared.lock().unwrap(); s.root.clone() }; let data = serde_json::to_vec(&load_pack_catalog(&root)).unwrap_or_else(|_| b"[]".to_vec()); write_response(&mut stream, "200 OK", "application/json; charset=utf-8", &data); return; }
    if path == "/obs.html" { write_response(&mut stream, "200 OK", "text/html; charset=utf-8", OBS_HTML.as_bytes()); return; }
    if path == "/api/state" {
        let state = { let s = shared.lock().unwrap(); let items: Vec<serde_json::Value> = s.config.slots.iter().enumerate().map(|(i, slot)| serde_json::json!({"slot":i+1,"name":slot.name,"active":slot.active,"pinned":slot.pinned,"duration_secs":slot.duration_secs,"layer":slot.layer,"html_file":slot.html_file,"url":slot.html_file.as_ref().and_then(|f|url_for_slot(&s.root,i,f))})).collect(); serde_json::json!({"revision":s.revision,"deckId":s.current_deck_id,"serverStatus":s.server_status,"slots":items,"instances":s.active_instances,"limits":{"maxActiveOverlays":MAX_ACTIVE_OVERLAYS},"cache":{"activeProtected":s.active_registry.active_instance_count(),"protectedResources":s.active_registry.protected_resource_count()},"memory":s.memory}) };
        let data = serde_json::to_vec(&state).unwrap_or_else(|_| b"{\"slots\":[]}".to_vec()); write_response(&mut stream, "200 OK", "application/json; charset=utf-8", &data); return;
    }
    if let Some(rest) = path.strip_prefix("/overlay/") {
        let mut parts = rest.splitn(2, '/'); let n = parts.next().unwrap_or(""); let rel = parts.next().unwrap_or("");
        let slot = match n.parse::<usize>() { Ok(n) if (1..=SLOT_COUNT).contains(&n) => n-1, _ => { write_response(&mut stream,"404 Not Found","text/plain",b"Unknown slot"); return; } };
        let decoded = decode_query_value(rel); let file = { let s=shared.lock().unwrap(); safe_slot_path(&s.root,slot,&decoded) };
        match file.and_then(|p| fs::read(&p).ok().map(|b|(p,b))) { Some((p,b))=>write_response(&mut stream,"200 OK",mime_type(&p),&b),None=>write_response(&mut stream,"404 Not Found","text/plain",b"Overlay file not found") }; return;
    }
    if let Some(rest) = path.strip_prefix("/packs/") {
        let decoded = decode_query_value(rest);
        let rel = match safe_relative_path(&decoded) { Some(p)=>p, None=>{write_response(&mut stream,"400 Bad Request","text/plain",b"Invalid path");return;} };
        let root = { let s = shared.lock().unwrap(); core_packs_root(&s.root) }; let file = root.join(rel);
        if !file.starts_with(&root) { write_response(&mut stream,"400 Bad Request","text/plain",b"Invalid path");return; }
        match fs::read(&file) { Ok(b)=>write_response(&mut stream,"200 OK",mime_type(&file),&b),Err(_)=>write_response(&mut stream,"404 Not Found","text/plain",b"Pack file not found") }; return;
    }
    write_response(&mut stream, "404 Not Found", "text/plain", b"Not found");
}

const UI_HTML: &str = include_str!("../ui/index.html");

// Bind synchronously so app startup knows whether the persistent OBS endpoint
// actually owns 127.0.0.1:8790 before the UI tries to navigate to it.
fn start_server(shared: Arc<Mutex<Shared>>) -> std::io::Result<()> {
    if let Ok(mut state) = shared.lock() { state.server_status = "STARTING".into(); }
    let listener = match TcpListener::bind(SERVER_ADDR) {
        Ok(listener) => listener,
        Err(error) => {
            if let Ok(mut state) = shared.lock() {
                state.server_status = "SERVER_FAILED".into();
                state.message = format!("StimTake OBS server could not start. Address: {SERVER_ADDR}. Error: {error}");
            }
            return Err(error);
        }
    };
    if let Err(error) = listener.set_nonblocking(false) {
        if let Ok(mut state) = shared.lock() {
            state.server_status = "SERVER_FAILED".into();
            state.message = format!("StimTake OBS server could not start. Address: {SERVER_ADDR}. Error: {error}");
        }
        return Err(error);
    }
    let server_state = shared.clone();
    let server_thread = thread::Builder::new()
        .name("stimtake-local-http".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let state = server_state.clone();
                        let _ = thread::Builder::new()
                            .name("stimtake-http-request".into())
                            .spawn(move || handle_http(stream, &state));
                    }
                    Err(error) => {
                        if let Ok(mut state) = server_state.lock() {
                            state.message = format!("Local OBS server accept error: {error}");
                        }
                    }
                }
            }
        });
    if let Err(error) = server_thread {
        if let Ok(mut state) = shared.lock() {
            state.server_status = "SERVER_FAILED".into();
            state.message = format!("StimTake OBS server could not start. Address: {SERVER_ADDR}. Error: {error}");
        }
        return Err(error);
    }
    if let Ok(mut state) = shared.lock() {
        state.server_status = "SERVER_READY".into();
        state.message = format!("OBS server ready at http://{SERVER_ADDR}/obs.html");
    }
    Ok(())
}

fn html_escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

const OBS_HTML: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<style>html,body{margin:0;width:100%;height:100%;overflow:hidden;background:transparent!important}#layers{position:relative;width:100vw;height:100vh;overflow:hidden;background:transparent}.layer{position:absolute;inset:0;width:100%;height:100%;border:0;background:transparent;pointer-events:auto}</style>
</head><body><div id="layers"></div><script>
const root=document.getElementById('layers');const frames=new Map();let revision=-1;
async function tick(){try{const r=await fetch('/api/state',{cache:'no-store'});if(!r.ok)throw Error('state');const s=await r.json();const active=(s.instances||[]).slice().sort((a,b)=>a.layer-b.layer);const wanted=new Set(active.map(x=>String(x.instance_id)));
for(const [id,frame] of frames){if(!wanted.has(id)){frame.remove();frames.delete(id);}}
for(const item of active){const id=String(item.instance_id);let frame=frames.get(id);if(!frame){frame=document.createElement('iframe');frame.className='layer';frame.title=item.name||('Overlay '+item.slot);frame.setAttribute('allowtransparency','true');frame.style.background='transparent';frame.dataset.overlayUrl=item.url;frames.set(id,frame);root.appendChild(frame);frame.src=item.url;}else if(frame.dataset.overlayUrl!==item.url){frame.dataset.overlayUrl=item.url;frame.src=item.url;}
frame.style.zIndex=String(item.layer);frame.title=item.name||('Overlay '+item.slot);}
revision=s.revision||revision;
}catch(e){/* keep existing layers during brief server hiccups */}finally{setTimeout(tick,250)}}tick();
</script></body></html>"#;

mod memory_monitor;
mod cache_manager;
mod runtime_limits;

fn main() -> wry::Result<()> {
    let root = app_root();
    let config = load_config(&root);
    let shared = Arc::new(Mutex::new(Shared {
        config, root: root.clone(), message: format!("App folder: {}", root.display()), server_status: "STARTING".into(), revision: 0,
        expirations: vec![None; SLOT_COUNT],
        memory: memory_monitor::MemoryStatus::default(),
        current_deck_id: None,
        active_instances: Vec::new(),
        next_instance_id: 0,
        active_registry: cache_manager::ActiveOverlayRegistry::default(),
    }));
    if let Ok(mut s) = shared.lock() { s.save_config(); }
    let server_start = start_server(shared.clone());
    if let Err(error) = &server_start {
        if let Ok(mut state) = shared.lock() {
            state.server_status = "SERVER_FAILED".into();
            state.message = format!("StimTake OBS server could not start. Address: {SERVER_ADDR}. Error: {error}");
        }
    }
    memory_monitor::start(shared.clone());

    let event_loop = tao::event_loop::EventLoop::new();
    let window = tao::window::WindowBuilder::new()
        .with_title("StimTake — Decks")
        .with_inner_size(tao::dpi::LogicalSize::new(1180.0, 820.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(760.0, 560.0))
        .build(&event_loop)
        .expect("Could not create StimTake window");
    let webview_builder = wry::WebViewBuilder::new();
    let _webview = match server_start {
        Ok(()) => webview_builder
            .with_url("http://127.0.0.1:8790/")
            .build(&window)?,
        Err(error) => {
            // Avoid presenting an apparently running app with a dead OBS URL.
            // The inline diagnostic page remains available even when port binding fails.
            let detail = html_escape(&error.to_string());
            let diagnostic = format!(r#"<!doctype html><html><head><meta charset="utf-8"><style>body{{font:16px Segoe UI,Arial;background:#111827;color:#f9fafb;padding:36px}}h1{{color:#f87171}}code{{color:#93c5fd}}</style></head><body><h1>StimTake local server did not start</h1><p>StimTake could not bind its local OBS server to <code>{SERVER_ADDR}</code>.</p><p>Close any other app using port 8790, then restart StimTake.</p><p>Details: {detail}</p><p>The OBS Browser Source at <code>http://127.0.0.1:8790/obs.html</code> will not work until this is fixed.</p></body></html>"#);
            webview_builder.with_html(&diagnostic).build(&window)?
        }
    };
    event_loop.run(move |event, _, control_flow| {
        *control_flow = tao::event_loop::ControlFlow::Wait;
        if let tao::event::Event::WindowEvent { event: tao::event::WindowEvent::CloseRequested, .. } = event {
            *control_flow = tao::event_loop::ControlFlow::Exit;
        }
    })
}
