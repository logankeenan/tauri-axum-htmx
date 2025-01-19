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
