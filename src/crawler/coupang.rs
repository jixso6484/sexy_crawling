use async_trait::async_trait;
use scraper::{Html, Selector};
use tracing::{debug, info, warn};

use crate::crawler::{create_http_client, Crawler};
use crate::error::{CrawlerError, Result};
use crate::types::{CrawlConfig, Product, ProductSource};

pub struct CoupangCrawler {
    client: reqwest::Client,
}

impl CoupangCrawler {
    pub fn new(user_agent: &str) -> Result<Self> {
        Ok(Self {
            client: create_http_client(user_agent)?,
        })
    }

    fn build_search_url(query: &str, page: u32) -> String {
        // 쿠팡 검색 URL 구조
        format!(
            "https://www.coupang.com/np/search?q={}&page={}",
            urlencoding::encode(query),
            page
        )
    }

    fn parse_products(&self, html: &str) -> Result<Vec<Product>> {
        let document = Html::parse_document(html);
        let mut products = Vec::new();

        // 쿠팡 상품 리스트 셀렉터 (실제 구조에 맞게 조정 필요)
        let product_selector = Selector::parse("li.search-product")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let name_selector = Selector::parse(".name")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let price_selector = Selector::parse(".price-value")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let discount_selector = Selector::parse(".discount-percentage")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let rating_selector = Selector::parse(".rating")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let image_selector = Selector::parse("img.search-product-wrap-img")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let link_selector = Selector::parse("a.search-product-link")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        for element in document.select(&product_selector) {
            let name = element
                .select(&name_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if name.is_empty() {
                continue;
            }

            let price = element
                .select(&price_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());

            let discount_rate = element
                .select(&discount_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());

            let rating_text = element
                .select(&rating_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());

            let rating = rating_text.and_then(|r| r.parse::<f32>().ok());

            let image_url = element
                .select(&image_selector)
                .next()
                .and_then(|e| e.value().attr("src"))
                .map(|s| s.to_string());

            let product_url = element
                .select(&link_selector)
                .next()
                .and_then(|e| e.value().attr("href"))
                .map(|href| {
                    if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("https://www.coupang.com{}", href)
                    }
                })
                .unwrap_or_default();

            if product_url.is_empty() {
                continue;
            }

            products.push(Product {
                name,
                price,
                original_price: None,
                discount_rate,
                rating,
                review_count: None,
                image_url,
                product_url,
                seller: None,
                delivery_info: None,
                source: ProductSource::Coupang,
                raw_data: None,
            });
        }

        Ok(products)
    }
}

#[async_trait]
impl Crawler for CoupangCrawler {
    fn name(&self) -> &str {
        "Coupang"
    }

    async fn crawl(&self, config: &CrawlConfig) -> Result<Vec<Product>> {
        info!("Starting Coupang crawl for query: {}", config.search_query);
        let mut all_products = Vec::new();

        for page in 1..=config.max_pages {
            debug!("Crawling Coupang page {}", page);

            let url = Self::build_search_url(&config.search_query, page);

            match self.client.get(&url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        warn!("Failed to fetch page {}: status {}", page, response.status());
                        continue;
                    }

                    match response.text().await {
                        Ok(html) => {
                            match self.parse_products(&html) {
                                Ok(mut products) => {
                                    info!("Found {} products on page {}", products.len(), page);
                                    all_products.append(&mut products);
                                }
                                Err(e) => {
                                    warn!("Failed to parse products on page {}: {}", page, e);
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read response text on page {}: {}", page, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch page {}: {}", page, e);
                }
            }

            // 요청 간 딜레이 (서버 부하 방지)
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        info!("Coupang crawl completed. Total products: {}", all_products.len());
        Ok(all_products)
    }

    async fn extract_product(&self, url: &str) -> Result<Option<Product>> {
        debug!("Extracting product from URL: {}", url);

        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Ok(None);
        }

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // 상세 페이지 파싱 로직 (기본 구현)
        let name_selector = Selector::parse("h2.prod-buy-header__title")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let name = document
            .select(&name_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        if name.is_empty() {
            return Ok(None);
        }

        Ok(Some(Product {
            name,
            price: None,
            original_price: None,
            discount_rate: None,
            rating: None,
            review_count: None,
            image_url: None,
            product_url: url.to_string(),
            seller: None,
            delivery_info: None,
            source: ProductSource::Coupang,
            raw_data: None,
        }))
    }
}
