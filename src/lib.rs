//! # Tauri Axum HTMX
//! A library for creating interactive UIs using HTMX and Axum within Tauri applications.
//! This crate provides the necessary infrastructure to handle HTMX requests through
//! Tauri's FFI bridge, process them using an Axum application, and return responses
//! back to the webview.
//! ## Overview
//! In a typical HTMX application, requests are sent to a server which returns HTML to
//! the client. This crate enables this pattern within Tauri applications by:
//! - Intercepting HTMX requests in the webview
//! - Forwarding them through Tauri's FFI bridge
//! - Processing them with an Axum application running in the Tauri backend
//! - Returning the response back to be handled by HTMX in the webview
//! ## Quick Start
//! 1. First, initialize the client-side integration in your HTML:
//! ```html
//! <!doctype html>
//! <html lang="en">
//!   <head>
//!     <script src="https://unpkg.com/htmx.org@2.0.4"></script>
//!     <script type="module">
//!       import { initialize } from "https://unpkg.com/htmx.org/tauri-axum-htmx";
//!
//!       initialize("/"); // the initial path for the application to start on
//!     </script>
//!   </head>
//! </html>
//! ```
//! 2. Then, set up the Tauri command to handle requests:
//! ```rust,no_run
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//! use axum::{Router, routing::get};
//! use tauri::State;
//! use tauri_axum_htmx::{LocalRequest, LocalResponse};
//! struct TauriState {
//!     router: Arc<Mutex<Router>>,
//! }
//! #[tauri::command]
//! async fn local_app_request(
//!     state: State<'_, TauriState>,
//!     local_request: LocalRequest,
//! ) -> Result<LocalResponse, ()> {
//!     let mut router = state.router.lock().await;
//!     let response = local_request.send_to_router(&mut router).await;
//!     Ok(response)
//! }
//! fn main() {
//!     let app = Router::new()
//!         .route("/", get(|| async { "Hello, World!" }));
//!     let tauri_state = TauriState {
//!         router: Arc::new(Mutex::new(app)),
//!     };
//!     tauri::Builder::default()
//!         .manage(tauri_state)
//!         .invoke_handler(tauri::generate_handler![local_app_request]);
//! }
//! ```

use axum::http::{self};
use axum::response::Response;
use axum::Router;
use axum::{body::Body, http::Request};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use thiserror::Error;
use tower_service::Service;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not parse method from LocalRequest")]
    RequestMethodParseError(String),

    #[error("Could not parse body from LocalRequest")]
    RequestBodyParseError(#[from] http::Error),
}

/// Represents an HTTP request that can be processed by an Axum router.
#[derive(Serialize, Deserialize, Clone)]
pub struct LocalRequest {
    pub uri: String,
    pub method: String,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
}

impl LocalRequest {
    pub async fn send_to_router(self, router: &mut Router) -> LocalResponse {
        match self.to_axum_request() {
            Ok(request) => match router.call(request).await {
                Ok(response) => LocalResponse::from_response(response).await,
                Err(error) => LocalResponse::internal_server_error(error),
            },
            Err(error) => LocalResponse::internal_server_error(error),
        }
    }

    fn to_axum_request(&self) -> Result<http::Request<Body>, Error> {
        let uri = self.uri.to_string();
        let mut request_builder = match self.method.to_uppercase().as_str() {
            "GET" => Ok(Request::get(uri)),
            "POST" => Ok(Request::post(uri)),
            "PUT" => Ok(Request::put(uri)),
            "DELETE" => Ok(Request::delete(uri)),
            "PATCH" => Ok(Request::patch(uri)),
            _ => Err(Error::RequestMethodParseError(self.method.to_string())),
        }?;

        for (key, value) in self.headers.iter() {
            request_builder = request_builder.header(key, value);
        }

        let request = match &self.body {
            None => request_builder.body(Body::empty()),
            Some(body) => request_builder.body(body.to_string().into()),
        }?;

        Ok(request)
    }
}

/// Represents an HTTP response returned from an Axum router.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalResponse {
    pub status_code: u16,
    pub body: Vec<u8>,
    pub headers: HashMap<String, String>,
}

impl LocalResponse {
    pub fn internal_server_error(error: impl Display) -> Self {
        let error_message = format!("An error occured: {}", error);
        LocalResponse {
            status_code: 500,
            body: error_message.into(),
            headers: Default::default(),
        }
    }
}

impl LocalResponse {
    pub async fn from_response(response: Response) -> Self {
        let code = response.status();
        let response_headers = response.headers().clone();
        let bytes_result = axum::body::to_bytes(response.into_body(), usize::MAX).await;

        let mut headers: HashMap<String, String> = HashMap::new();
        for (key, value) in response_headers.iter() {
            headers.insert(key.to_string(), value.to_str().unwrap().to_string());
        }

        match bytes_result {
            Ok(data) => LocalResponse {
                status_code: code.as_u16(),
                body: data.to_vec(),
                headers,
            },
            Err(_) => LocalResponse {
                status_code: code.as_u16(),
                body: Vec::new(),
                headers: headers.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        routing::{get, post},
        Json,
    };
    use serde_json::json;

    // Helper function to create a basic router for testing
    fn create_test_router() -> Router {
        Router::new()
            .route("/test", get(|| async { "Hello, World!" }))
            .route("/echo", post(|body: String| async move { body }))
            .route("/json", get(|| async { Json(json!({"status": "ok"})) }))
    }

    mod local_request_tests {
        use super::*;

        #[tokio::test]
        async fn test_basic_get_request() {
            let mut router = create_test_router();
            let request = LocalRequest {
                uri: "/test".to_string(),
                method: "GET".to_string(),
                body: None,
                headers: HashMap::new(),
            };

            let response = request.send_to_router(&mut router).await;
            assert_eq!(response.status_code, 200);
            assert_eq!(String::from_utf8(response.body).unwrap(), "Hello, World!");
        }

        #[tokio::test]
        async fn test_post_request_with_body() {
            let mut router = create_test_router();
            let body = "Test Body";
            let request = LocalRequest {
                uri: "/echo".to_string(),
                method: "POST".to_string(),
                body: Some(body.to_string()),
                headers: HashMap::new(),
            };

            let response = request.send_to_router(&mut router).await;
            assert_eq!(response.status_code, 200);
            assert_eq!(String::from_utf8(response.body).unwrap(), body);
        }

        #[tokio::test]
        async fn test_invalid_method() {
            let mut router = create_test_router();
            let request = LocalRequest {
                uri: "/test".to_string(),
                method: "INVALID".to_string(),
                body: None,
                headers: HashMap::new(),
            };

            let response = request.send_to_router(&mut router).await;
            assert_eq!(response.status_code, 500);
            assert!(String::from_utf8(response.body)
                .unwrap()
                .contains("Could not parse method"));
        }

        #[tokio::test]
        async fn test_request_with_headers() {
            let mut router = Router::new().route(
                "/headers",
                get(|req: Request<Body>| async move {
                    let header_value = req
                        .headers()
                        .get("X-Test-Header")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    header_value
                }),
            );

            let mut headers = HashMap::new();
            headers.insert("X-Test-Header".to_string(), "test-value".to_string());

            let request = LocalRequest {
                uri: "/headers".to_string(),
                method: "GET".to_string(),
                body: None,
                headers,
            };

            let response = request.send_to_router(&mut router).await;
            assert_eq!(response.status_code, 200);
            assert_eq!(String::from_utf8(response.body).unwrap(), "test-value");
        }
    }

    mod local_response_tests {
        use super::*;
        use http::response::Builder;

        #[tokio::test]
        async fn test_response_creation_with_body() {
            let response = Builder::new()
                .status(200)
                .body(Body::from("test body"))
                .unwrap();

            let local_response = LocalResponse::from_response(response).await;
            assert_eq!(local_response.status_code, 200);
            assert_eq!(String::from_utf8(local_response.body).unwrap(), "test body");
        }

        #[tokio::test]
        async fn test_response_with_headers() {
            let response = Builder::new()
                .status(200)
                .header("X-Test", "test-value")
                .body(Body::empty())
                .unwrap();

            let local_response = LocalResponse::from_response(response).await;
            assert_eq!(local_response.status_code, 200);
            assert_eq!(local_response.headers.get("x-test").unwrap(), "test-value");
        }

        #[tokio::test]
        async fn test_internal_server_error() {
            let error_message = "Test error";
            let response = LocalResponse::internal_server_error(error_message);

            assert_eq!(response.status_code, 500);
            assert!(String::from_utf8(response.body)
                .unwrap()
                .contains(error_message));
            assert!(response.headers.is_empty());
        }
    }

    mod method_tests {
        use super::*;

        #[tokio::test]
        async fn test_all_valid_methods() {
            let methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH"];

            for method in methods {
                let request = LocalRequest {
                    uri: "/test".to_string(),
                    method: method.to_string(),
                    body: None,
                    headers: HashMap::new(),
                };

                assert!(request.to_axum_request().is_ok());
            }
        }

        #[tokio::test]
        async fn test_method_case_insensitivity() {
            let request = LocalRequest {
                uri: "/test".to_string(),
                method: "get".to_string(),
                body: None,
                headers: HashMap::new(),
            };

            assert!(request.to_axum_request().is_ok());
        }
    }
}
