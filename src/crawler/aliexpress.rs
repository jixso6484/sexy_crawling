use async_trait::async_trait;
use scraper::{Html, Selector};
use tracing::{debug, info, warn};

use crate::crawler::{create_http_client, Crawler};
use crate::error::{CrawlerError, Result};
use crate::types::{CrawlConfig, Product, ProductSource};

pub struct AliExpressCrawler {
    client: reqwest::Client,
}

impl AliExpressCrawler {
    pub fn new(user_agent: &str) -> Result<Self> {
        Ok(Self {
            client: create_http_client(user_agent)?,
        })
    }

    fn build_search_url(query: &str, page: u32) -> String {
        // 알리익스프레스 검색 URL 구조
        format!(
            "https://www.aliexpress.com/wholesale?SearchText={}&page={}",
            urlencoding::encode(query),
            page
        )
    }

    fn parse_products(&self, html: &str) -> Result<Vec<Product>> {
        let document = Html::parse_document(html);
        let mut products = Vec::new();

        // 알리익스프레스 상품 리스트 셀렉터 (실제 구조에 맞게 조정 필요)
        // 알리익스프레스는 자주 구조가 변경되므로 여러 셀렉터 시도
        let product_selectors = vec![
            "div.list-item",
            "div.product-item",
            "a.item-anchor",
        ];

        let mut product_selector = None;
        for selector_str in product_selectors {
            if let Ok(sel) = Selector::parse(selector_str) {
                if document.select(&sel).next().is_some() {
                    product_selector = Some(sel);
                    break;
                }
            }
        }

        let product_selector = product_selector
            .ok_or_else(|| CrawlerError::ParseError("No product selector found".to_string()))?;

        let name_selector = Selector::parse(".item-title, .product-title, h1, h3")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let price_selector = Selector::parse(".price-current, .product-price-value, .price")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let rating_selector = Selector::parse(".rating-value, .stars")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let review_selector = Selector::parse(".review-count, .rating-num")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let image_selector = Selector::parse("img")
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

            // 링크 추출
            let product_url = if let Some(href) = element.value().attr("href") {
                if href.starts_with("http") {
                    href.to_string()
                } else if href.starts_with("//") {
                    format!("https:{}", href)
                } else {
                    format!("https://www.aliexpress.com{}", href)
                }
            } else {
                // a 태그 내부에서 링크 찾기
                if let Ok(link_sel) = Selector::parse("a") {
                    element
                        .select(&link_sel)
                        .next()
                        .and_then(|e| e.value().attr("href"))
                        .map(|href| {
                            if href.starts_with("http") {
                                href.to_string()
                            } else if href.starts_with("//") {
                                format!("https:{}", href)
                            } else {
                                format!("https://www.aliexpress.com{}", href)
                            }
                        })
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            };

            if product_url.is_empty() {
                continue;
            }

            let price = element
                .select(&price_selector)
                .next()
                .map(|e| {
                    e.text()
                        .collect::<String>()
                        .trim()
                        .replace("$", "")
                        .replace(",", "")
                        .trim()
                        .to_string()
                });

            let rating_text = element
                .select(&rating_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());

            let rating = rating_text.and_then(|r| r.parse::<f32>().ok());

            let review_text = element
                .select(&review_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());

            let review_count = review_text.and_then(|r| {
                r.chars()
                    .filter(|c| c.is_numeric())
                    .collect::<String>()
                    .parse::<u32>()
                    .ok()
            });

            let image_url = element
                .select(&image_selector)
                .next()
                .and_then(|e| {
                    e.value()
                        .attr("src")
                        .or_else(|| e.value().attr("data-src"))
                })
                .map(|s| {
                    if s.starts_with("http") {
                        s.to_string()
                    } else if s.starts_with("//") {
                        format!("https:{}", s)
                    } else {
                        format!("https:{}", s)
                    }
                });

            products.push(Product {
                name,
                price,
                original_price: None,
                discount_rate: None,
                rating,
                review_count,
                image_url,
                product_url,
                seller: None,
                delivery_info: None,
                source: ProductSource::AliExpress,
                raw_data: None,
            });
        }

        Ok(products)
    }
}

#[async_trait]
impl Crawler for AliExpressCrawler {
    fn name(&self) -> &str {
        "AliExpress"
    }

    async fn crawl(&self, config: &CrawlConfig) -> Result<Vec<Product>> {
        info!("Starting AliExpress crawl for query: {}", config.search_query);
        let mut all_products = Vec::new();

        for page in 1..=config.max_pages {
            debug!("Crawling AliExpress page {}", page);

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

            // 요청 간 딜레이
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }

        info!("AliExpress crawl completed. Total products: {}", all_products.len());
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

        let name_selector = Selector::parse("h1.product-title-text, h1")
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
            source: ProductSource::AliExpress,
            raw_data: None,
        }))
    }
}
