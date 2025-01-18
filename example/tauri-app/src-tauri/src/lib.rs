use std::sync::Arc;

use axum::Router;
use axum_app::create_axum_app;
use tauri::{async_runtime::Mutex, State};
use tauri_axum_htmx::{LocalRequest, LocalResponse};
use tower_service::Service;

struct AppState {
    router: Arc<Mutex<Router>>,
}

#[tauri::command]
async fn local_app_request(
    state: State<'_, AppState>,
    local_request: LocalRequest,
) -> Result<LocalResponse, ()> {
    let mut router = state.router.lock().await;

    match local_request.to_axum_request() {
        Ok(request) => match router.call(request).await {
            Ok(response) => Ok(LocalResponse::from_response(response).await),
            Err(error) => Ok(LocalResponse::internal_server_error(error)),
        },
        Err(error) => Ok(LocalResponse::internal_server_error(error)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let router = create_axum_app();
    
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
