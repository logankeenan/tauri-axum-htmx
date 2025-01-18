use axum::http::{self, StatusCode};
use axum::response::Response;
use axum::{body::Body, http::Request};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

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
    pub fn to_axum_request(&self) -> Result<http::Request<Body>, Error> {
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

impl From<LocalResponse> for Response<Body> {
    fn from(local_response: LocalResponse) -> Response<Body> {
        let mut response_builder = Response::builder().status(local_response.status_code);

        for (key, value) in local_response.headers.iter() {
            response_builder = response_builder.header(key, value);
        }

        let response = match local_response.body.is_empty() {
            true => response_builder.body(Body::empty()),
            false => response_builder.body(local_response.body.into()),
        };
        
        match response {
            Ok(response) => response,
            Err(_) => {
                let mut internal_server_error = Response::new(Body::empty());
                *internal_server_error.status_mut()  = StatusCode::INTERNAL_SERVER_ERROR;
                
                internal_server_error
            }
        }
    }
}