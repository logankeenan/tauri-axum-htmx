# Tauri Axum HTMX

Create interactive UIs using HTMX and Axum for Tauri applications.

Typically, Htmx sends requests to the server which returns HTML to the client. In this case,
the server is the Tauri rust backend running an Axum application. Requests are
sent over the Tauri FFI bridge to be processed by the Axum application and the
response is sent back to the webview to be updated by Htmx.

## Getting started

Create a vanilla Tauri project and initialize tauri-axum-htmx in src/index.html

```html
<!doctype html>
<html lang="en">
  <head>
    <script src="https://unpkg.com/htmx.org@2.0.4"></script>
    <script type="module">
      import { initialize } from "/vendor/tauri-axum-htmx.js";

      initialize("/"); // the initial path for the application to start on
    </script
  </head>
</html>
```

Create a Tauri command to process requests from the webview

```rust
struct TauriState {
    router: Arc<Mutex<Router>>,
}

#[tauri::command]
async fn local_app_request(
    state: State<'_, TauriState>,
    local_request: LocalRequest,
) -> Result<LocalResponse, ()> {
    let mut router = state.router.lock().await;

    let response = local_request.send_to_router(&mut router).await;

    Ok(response)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    let tauri_stat = TauriState {
        router: Arc::new(Mutex::new(router)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(tauri_stat)
        .invoke_handler(tauri::generate_handler![local_app_request])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## JavaScript API

`initialize(initialPath: string, localAppRequestCommandOverride: string)`

- `initialPath`: The initial path for the application to start on
- `localAppRequestCommandOverride`: The name of the Tauri command to process requests from the webview



