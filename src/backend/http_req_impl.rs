use std::convert::{Infallible, TryFrom};

use crate::models::{error::ErrorResponse, Object};
use crate::{TClient, NOTION_API_VERSION};

use async_trait::async_trait;
use http_req::error as hr_error;
use http_req::request::{Method, Request};
use http_req::uri::Uri;

/// An wrapper Error type for all errors produced by the [`NotionApi`](NotionApi) client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid Notion API Token: {}", source)]
    InvalidApiToken { source: hr_error::ParseErr },

    #[error("Unable to build reqwest HTTP client: {}", source)]
    ErrorBuildingClient { source: hr_error::Error },

    #[error("Error sending HTTP request: {}", source)]
    RequestFailed {
        #[from]
        source: hr_error::Error,
    },

    #[error("Error reading response: {}", source)]
    ResponseIoError { source: hr_error::Error },

    #[error("Error parsing json response: {}", source)]
    JsonParseError { source: serde_json::Error },

    #[error("Unexpected API Response")]
    UnexpectedResponse { response: Object },

    #[error("API Error {}({}): {}", .error.code, .error.status, .error.message)]
    ApiError { error: ErrorResponse },

    #[error("Infallible")]
    Infallible(#[from] Infallible),
}

/// An API client for Notion.
/// Create a client by using [new(api_token: String)](Self::new()).
#[derive(Clone)]
pub struct Client {
    token: String,
}

impl Client {
    /// Creates an instance of NotionApi.
    /// Never fail.
    pub fn new(api_token: String) -> Result<Self, Infallible> {
        Ok(Self { token: api_token })
    }
}

#[async_trait]
impl TClient for Client {
    async fn get<S: Into<String> + Send>(
        &self,
        uri: S,
    ) -> crate::Result<Object> {
        let raw: String = uri.into();

        let uri = Uri::try_from(raw.as_str()).unwrap();
        let mut request = Request::new(&uri);
        request.method(Method::GET);
        self.make_json_request(&mut request).await
    }

    async fn post<S: Into<String> + Send>(
        &self,
        uri: S,
    ) -> crate::Result<Object> {
        let raw: String = uri.into();

        let uri = Uri::try_from(raw.as_str()).unwrap();
        let mut request = Request::new(&uri);
        request.method(Method::POST);
        self.make_json_request(&mut request).await
    }

    async fn post_json<S: Into<String> + Send>(
        &self,
        uri: S,
        body: &[u8],
    ) -> crate::Result<Object> {
        let raw: String = uri.into();

        let uri = Uri::try_from(raw.as_str()).unwrap();
        let mut request = Request::new(&uri);
        request
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .header("Content-Length", &body.len())
            .body(body);

        self.make_json_request(&mut request).await
    }
}

impl Client {
    async fn make_json_request(
        &self,
        request: &mut Request<'_>,
    ) -> Result<Object, Error> {
        let mut writer = Vec::new();
        let resp = request
            .header("Notion-Version", NOTION_API_VERSION)
            .header("Authorization", &format!("Bearer {}", self.token))
            .send(&mut writer)
            .map_err(|source| Error::RequestFailed { source })?;

        let text = String::from_utf8_lossy(&writer);

        tracing::debug!("Response: {:?}", resp);
        #[cfg(test)]
        {
            dbg!(serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|source| Error::JsonParseError { source })?);
        }
        let result =
            serde_json::from_str(&text).map_err(|source| Error::JsonParseError { source })?;

        match result {
            Object::Error { error } => Err(Error::ApiError { error }),
            response => Ok(response),
        }
    }
}
