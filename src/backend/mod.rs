use crate::ids::{AsIdentifier, BlockId, DatabaseId, PageId};
use crate::models::{
    block::Block,
    search::{DatabaseQuery, SearchRequest},
    Database, ListResponse, Object, Page, PageCreateRequest,
};
use async_trait::async_trait;

#[cfg(not(target_os = "wasi"))]
mod reqwest_impl;

#[cfg(not(target_os = "wasi"))]
pub use reqwest_impl::{Client, Error};

#[cfg(target_os = "wasi")]
mod http_req_impl;

#[cfg(target_os = "wasi")]
pub use http_req_impl::{Client, Error};

pub type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait TClient {
    async fn get<S: Into<String> + Send>(
        &self,
        uri: S,
    ) -> Result<Object>;

    async fn post<S: Into<String> + Send>(
        &self,
        uri: S,
    ) -> Result<Object>;

    async fn post_json<S: Into<String> + Send>(
        &self,
        uri: S,
        body: &[u8],
    ) -> Result<Object>;
}

pub struct NotionApi {
    client: Client,
}

impl NotionApi {
    pub fn new<S: Into<String>>(api_token: S) -> Result<Self> {
        let client = Client::new(api_token.into())?;

        Ok(Self { client })
    }
}

impl NotionApi {
    /// List all the databases shared with the supplied integration token.
    /// > This method is apparently deprecated/"not recommended" and
    /// > [search()](Self::search()) should be used instead.
    pub async fn list_databases(&self) -> Result<ListResponse<Database>> {
        match self
            .client
            .get("https://api.notion.com/v1/databases")
            .await?
        {
            Object::List { list } => Ok(list.expect_databases()?),
            response => Err(Error::UnexpectedResponse { response }),
        }
    }

    /// Search all pages in notion.
    /// `query` can either be a [SearchRequest] or a slightly more convenient
    /// [NotionSearch](models::search::NotionSearch) query.
    pub async fn search<T: Into<SearchRequest>>(
        &self,
        query: T,
    ) -> Result<ListResponse<Object>> {
        let query = serde_json::to_string(&query.into()).unwrap();

        let result = self
            .client
            .post_json("https://api.notion.com/v1/search", query.as_bytes())
            .await?;

        match result {
            Object::List { list } => Ok(list),
            response => Err(Error::UnexpectedResponse { response }),
        }
    }

    /// Get a database by [DatabaseId].
    pub async fn get_database<T: AsIdentifier<DatabaseId>>(
        &self,
        database_id: T,
    ) -> Result<Database> {
        let uri = format!(
            "https://api.notion.com/v1/databases/{}",
            database_id.as_id()
        );
        let result = self.client.get(uri).await?;

        match result {
            Object::Database { database } => Ok(database),
            response => Err(Error::UnexpectedResponse { response }),
        }
    }

    /// Get a page by [PageId].
    pub async fn get_page<T: AsIdentifier<PageId>>(
        &self,
        page_id: T,
    ) -> Result<Page> {
        let uri = format!("https://api.notion.com/v1/pages/{}", page_id.as_id());
        let result = self.client.get(uri).await?;

        match result {
            Object::Page { page } => Ok(page),
            response => Err(Error::UnexpectedResponse { response }),
        }
    }

    /// Creates a new page and return the created page
    pub async fn create_page<T: Into<PageCreateRequest>>(
        &self,
        page: T,
    ) -> Result<Page> {
        let page = serde_json::to_string(&page.into()).unwrap();

        let result = self
            .client
            .post_json("https://api.notion.com/v1/pages", page.as_bytes())
            .await?;

        match result {
            Object::Page { page } => Ok(page),
            response => Err(Error::UnexpectedResponse { response }),
        }
    }

    /// Query a database and return the matching pages.
    pub async fn query_database<D, T>(
        &self,
        database: D,
        query: T,
    ) -> Result<ListResponse<Page>>
    where
        T: Into<DatabaseQuery>,
        D: AsIdentifier<DatabaseId>,
    {
        let query = serde_json::to_string(&query.into()).unwrap();

        let uri = format!(
            "https://api.notion.com/v1/databases/{database_id}/query",
            database_id = database.as_id()
        );

        let result = self.client.post_json(uri, query.as_bytes()).await?;

        match result {
            Object::List { list } => Ok(list.expect_pages()?),
            response => Err(Error::UnexpectedResponse { response }),
        }
    }

    pub async fn get_block_children<T: AsIdentifier<BlockId>>(
        &self,
        block_id: T,
    ) -> Result<ListResponse<Block>> {
        let uri = format!(
            "https://api.notion.com/v1/blocks/{block_id}/children",
            block_id = block_id.as_id()
        );

        let result = self.client.get(uri).await?;

        match result {
            Object::List { list } => Ok(list.expect_blocks()?),
            response => Err(Error::UnexpectedResponse { response }),
        }
    }
}
