use async_trait::async_trait;
use scraper::{Html, Selector};
use tracing::{debug, info, warn};

use crate::crawler::{create_http_client, Crawler};
use crate::error::{CrawlerError, Result};
use crate::types::{CrawlConfig, Product, ProductSource};

pub struct DanawaCrawler {
    client: reqwest::Client,
}

impl DanawaCrawler {
    pub fn new(user_agent: &str) -> Result<Self> {
        Ok(Self {
            client: create_http_client(user_agent)?,
        })
    }

    fn build_search_url(query: &str, page: u32) -> String {
        // 다나와 검색 URL 구조
        let page_num = (page - 1) * 30; // 다나와는 보통 30개씩
        format!(
            "https://search.danawa.com/dsearch.php?query={}&page={}&limit=30",
            urlencoding::encode(query),
            page_num
        )
    }

    fn parse_products(&self, html: &str) -> Result<Vec<Product>> {
        let document = Html::parse_document(html);
        let mut products = Vec::new();

        // 다나와 상품 리스트 셀렉터 (실제 구조에 맞게 조정 필요)
        let product_selector = Selector::parse("div.prod_item, li.prod_item")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let name_selector = Selector::parse(".prod_name a, .prod_info a")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let price_selector = Selector::parse(".price_sect strong, .price em")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let image_selector = Selector::parse(".thumb_image img, .prod_img img")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        let seller_selector = Selector::parse(".mall_name, .seller")
            .map_err(|e| CrawlerError::SelectorError(e.to_string()))?;

        for element in document.select(&product_selector) {
            let name_elem = element.select(&name_selector).next();

            let name = name_elem
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            if name.is_empty() {
                continue;
            }

            let product_url = name_elem
                .and_then(|e| e.value().attr("href"))
                .map(|href| {
                    if href.starts_with("http") {
                        href.to_string()
                    } else if href.starts_with("//") {
                        format!("https:{}", href)
                    } else {
                        format!("https://search.danawa.com{}", href)
                    }
                })
                .unwrap_or_default();

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
                        .replace(",", "")
                        .replace("원", "")
                        .trim()
                        .to_string()
                });

            let image_url = element
                .select(&image_selector)
                .next()
                .and_then(|e| e.value().attr("src").or_else(|| e.value().attr("data-original")))
                .map(|s| {
                    if s.starts_with("http") {
                        s.to_string()
                    } else if s.starts_with("//") {
                        format!("https:{}", s)
                    } else {
                        format!("https://search.danawa.com{}", s)
                    }
                });

            let seller = element
                .select(&seller_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string());

            products.push(Product {
                name,
                price,
                original_price: None,
                discount_rate: None,
                rating: None,
                review_count: None,
                image_url,
                product_url,
                seller,
                delivery_info: None,
                source: ProductSource::Danawa,
                raw_data: None,
            });
        }

        Ok(products)
    }
}

#[async_trait]
impl Crawler for DanawaCrawler {
    fn name(&self) -> &str {
        "Danawa"
    }

    async fn crawl(&self, config: &CrawlConfig) -> Result<Vec<Product>> {
        info!("Starting Danawa crawl for query: {}", config.search_query);
        let mut all_products = Vec::new();

        for page in 1..=config.max_pages {
            debug!("Crawling Danawa page {}", page);

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
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        info!("Danawa crawl completed. Total products: {}", all_products.len());
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

        let name_selector = Selector::parse(".prod_tit, .top_summary h3")
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
            source: ProductSource::Danawa,
            raw_data: None,
        }))
    }
}
