use std::str::FromStr;

use notion_wasi::{ids::PageId, NotionApi};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let token = env!("NOTION_TOKEN");

    let api = NotionApi::new(token).unwrap();

    let page_id = PageId::from_str("25169f061d4447b2a0232ab35d752a5f").unwrap();
    let a = api.get_page(page_id).await;

    println!("{:?}", a);
}
