use std::sync::Arc;

use axum::Router;
use axum_app::create_axum_app;
use tauri::{async_runtime::Mutex, State};
use tauri_axum_htmx::{LocalRequest, LocalResponse};

struct AppState {
    router: Arc<Mutex<Router>>,
}

#[tauri::command]
async fn local_app_request(
    state: State<'_, AppState>,
    local_request: LocalRequest,
) -> Result<LocalResponse, ()> {
    let mut router = state.router.lock().await;

    let response = local_request.send_to_router(&mut router).await;

    Ok(response)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let router: Router = create_axum_app();

    let app_state = AppState {
        router: Arc::new(Mutex::new(router)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![local_app_request])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
