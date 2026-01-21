use collections::HashSet;
use gpui::{App, AppContext, WeakEntity};
use lsp::{LanguageServer, LanguageServerName, Uri};
use parking_lot::Mutex;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

use crate::LspStore;

pub const GOPLS_SERVER_NAME: LanguageServerName = LanguageServerName::new_static("gopls");

struct GoplsGcDetailsState {
    enabled_packages: HashSet<PathBuf>,
}

static GC_DETAILS_STATE: Mutex<Option<GoplsGcDetailsState>> = Mutex::new(None);

fn get_or_init_state() -> &'static Mutex<Option<GoplsGcDetailsState>> {
    {
        let mut state = GC_DETAILS_STATE.lock();
        if state.is_none() {
            *state = Some(GoplsGcDetailsState {
                enabled_packages: HashSet::default(),
            });
        }
    }
    &GC_DETAILS_STATE
}

pub fn enable_gc_details_for_buffer(
    language_server: &Arc<LanguageServer>,
    file_uri: &Uri,
    file_path: &PathBuf,
    cx: &mut App,
) {
    if language_server.name() != GOPLS_SERVER_NAME {
        return;
    }

    let package_dir = match file_path.parent() {
        Some(dir) => dir.to_path_buf(),
        None => return,
    };

    let should_enable = {
        let state_mutex = get_or_init_state();
        let mut state_guard = state_mutex.lock();
        if let Some(state) = state_guard.as_mut() {
            if state.enabled_packages.contains(&package_dir) {
                false
            } else {
                state.enabled_packages.insert(package_dir);
                true
            }
        } else {
            false
        }
    };

    if !should_enable {
        return;
    }

    let server = language_server.clone();
    let uri_string = file_uri.to_string();

    cx.background_spawn(async move {
        let result = server
            .request::<lsp::request::ExecuteCommand>(lsp::ExecuteCommandParams {
                command: "gopls.gc_details".to_string(),
                arguments: vec![json!(uri_string)],
                ..Default::default()
            })
            .await;

        match result {
            util::ConnectionResult::Result(Ok(_)) => {
                log::debug!(
                    "Enabled gc_details for gopls package containing {}",
                    uri_string
                );
            }
            util::ConnectionResult::Result(Err(err)) => {
                log::warn!("Failed to enable gc_details for gopls: {:?}", err);
            }
            other => {
                log::debug!("gc_details command result: {:?}", other);
            }
        }
    })
    .detach();
}

pub fn register_notifications(_lsp_store: WeakEntity<LspStore>, language_server: &LanguageServer) {
    if language_server.name() != GOPLS_SERVER_NAME {
        return;
    }

    get_or_init_state();
}

#[allow(dead_code)]
pub fn clear_state_for_server(language_server_name: &LanguageServerName) {
    if *language_server_name == GOPLS_SERVER_NAME {
        let mut state = GC_DETAILS_STATE.lock();
        if let Some(s) = state.as_mut() {
            s.enabled_packages.clear();
        }
    }
}
