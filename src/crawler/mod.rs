pub mod aliexpress;
pub mod coupang;
pub mod danawa;
pub mod google;
pub mod utils;

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

/// 재시도 로직을 포함한 HTTP GET 요청
pub async fn fetch_with_retry(
    client: &reqwest::Client,
    url: &str,
    max_retries: u32,
) -> Result<String> {
    let mut last_error = None;

    for attempt in 0..max_retries {
        if attempt > 0 {
            // 재시도 전 대기 (exponential backoff)
            let delay = std::time::Duration::from_millis(500 * (2_u64.pow(attempt - 1)));
            tokio::time::sleep(delay).await;
        }

        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(text) => return Ok(text),
                        Err(e) => last_error = Some(e.into()),
                    }
                } else {
                    last_error = Some(crate::error::CrawlerError::HttpError(
                        reqwest::Error::from(response.error_for_status().unwrap_err()),
                    ));
                }
            }
            Err(e) => last_error = Some(e.into()),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        crate::error::CrawlerError::Unknown("Max retries exceeded".to_string())
    }))
}
