use crate::models::{error::ErrorResponse, Object};
use crate::{TClient, NOTION_API_VERSION};

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{header, Client as RClient, ClientBuilder, RequestBuilder};
use tracing::Instrument;

/// An wrapper Error type for all errors produced by the [`NotionApi`](NotionApi) client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid Notion API Token: {}", source)]
    InvalidApiToken { source: header::InvalidHeaderValue },

    #[error("Unable to build reqwest HTTP client: {}", source)]
    ErrorBuildingClient { source: reqwest::Error },

    #[error("Error sending HTTP request: {}", source)]
    RequestFailed {
        #[from]
        source: reqwest::Error,
    },

    #[error("Error reading response: {}", source)]
    ResponseIoError { source: reqwest::Error },

    #[error("Error parsing json response: {}", source)]
    JsonParseError { source: serde_json::Error },

    #[error("Unexpected API Response")]
    UnexpectedResponse { response: Object },

    #[error("API Error {}({}): {}", .error.code, .error.status, .error.message)]
    ApiError { error: ErrorResponse },
}

/// An API client for Notion.
/// Create a client by using [new(api_token: String)](Self::new()).
#[derive(Clone)]
pub struct Client {
    client: RClient,
}

impl Client {
    pub fn new(api_token: String) -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Notion-Version",
            HeaderValue::from_static(NOTION_API_VERSION),
        );

        let mut auth_value = HeaderValue::from_str(&format!("Bearer {}", api_token))
            .map_err(|source| Error::InvalidApiToken { source })?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        let client = ClientBuilder::new()
            .default_headers(headers)
            .build()
            .map_err(|source| Error::ErrorBuildingClient { source })?;

        Ok(Self { client })
    }
}

#[async_trait]
impl TClient for Client {
    async fn get<S: Into<String> + Send>(
        &self,
        uri: S,
    ) -> crate::Result<Object> {
        let url: String = uri.into();

        let request = self.client.get(url);
        self.make_json_request(request).await
    }

    async fn post<S: Into<String> + Send>(
        &self,
        uri: S,
    ) -> crate::Result<Object> {
        let url: String = uri.into();

        let request = self.client.post(url);
        self.make_json_request(request).await
    }

    async fn post_json<S: Into<String> + Send>(
        &self,
        uri: S,
        body: &[u8],
    ) -> crate::Result<Object> {
        let url: String = uri.into();

        let request = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Content-Length", body.len())
            .body(body.to_owned());

        self.make_json_request(request).await
    }
}

impl Client {
    async fn make_json_request(
        &self,
        request: RequestBuilder,
    ) -> Result<Object, Error> {
        let request = request.build()?;
        let url = request.url();
        tracing::trace!(
            method = request.method().as_str(),
            url = url.as_str(),
            "Sending request"
        );
        let json = self
            .client
            .execute(request)
            .instrument(tracing::trace_span!("Sending request"))
            .await
            .map_err(|source| Error::RequestFailed { source })?
            .text()
            .instrument(tracing::trace_span!("Reading response"))
            .await
            .map_err(|source| Error::ResponseIoError { source })?;

        tracing::debug!("JSON Response: {}", json);
        #[cfg(test)]
        {
            dbg!(serde_json::from_str::<serde_json::Value>(&json)
                .map_err(|source| Error::JsonParseError { source })?);
        }
        let result =
            serde_json::from_str(&json).map_err(|source| Error::JsonParseError { source })?;

        match result {
            Object::Error { error } => Err(Error::ApiError { error }),
            response => Ok(response),
        }
    }
}
