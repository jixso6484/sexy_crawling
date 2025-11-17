pub mod coupang;
pub mod danawa;
pub mod aliexpress;

use async_trait::async_trait;
use crate::error::Result;
use crate::types::{CrawlConfig, Product};

#[async_trait]
pub trait Crawler: Send + Sync {
    /// 크롤러 이름 반환
    fn name(&self) -> &str;

    /// 검색 쿼리로 상품 크롤링
    async fn crawl(&self, config: &CrawlConfig) -> Result<Vec<Product>>;

    /// 특정 URL에서 상품 정보 추출
    async fn extract_product(&self, url: &str) -> Result<Option<Product>>;
}

/// HTTP 클라이언트 생성 유틸리티
pub fn create_http_client(user_agent: &str) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(30))
        .cookie_store(true)
        .build()?)
}
