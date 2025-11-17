use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, info, warn};

use crate::crawler::{create_http_client, fetch_with_retry, utils, Crawler};
use crate::error::Result;
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
        format!(
            "https://www.aliexpress.com/wholesale?SearchText={}&page={}",
            urlencoding::encode(query),
            page
        )
    }

    fn extract_product_from_element(&self, element: ElementRef) -> Option<Product> {
        // 상품명 추출 - 여러 셀렉터 시도
        let name_selectors = vec![
            ".item-title",
            ".product-title",
            "h1",
            "h3",
            "h2",
            "[class*='title']",
            "a[title]",
        ];

        let name = self
            .try_extract_text(&element, &name_selectors)
            .or_else(|| {
                // title 속성에서 추출
                self.try_extract_attr(&element, &["a"], "title")
            })?;

        if name.is_empty() {
            return None;
        }

        // 링크 추출
        let product_url = if let Some(href) = element.value().attr("href") {
            href.to_string()
        } else {
            let link_selectors = vec!["a", "a.item-anchor", "a[href]"];
            self.try_extract_attr(&element, &link_selectors, "href")?
        };

        if product_url.is_empty() {
            return None;
        }

        // 가격 추출
        let price_selectors = vec![
            ".price-current",
            ".product-price-value",
            ".price",
            "[class*='price']",
            "span.price",
        ];
        let price = self.try_extract_text(&element, &price_selectors).map(|p| {
            p.replace("$", "")
                .replace(",", "")
                .replace("US", "")
                .trim()
                .to_string()
        });

        // 평점 추출
        let rating_selectors = vec![
            ".rating-value",
            ".stars",
            "[class*='rating']",
            "[class*='star']",
        ];
        let rating_text = self.try_extract_text(&element, &rating_selectors);
        let rating = rating_text.and_then(|r| {
            r.chars()
                .filter(|c| c.is_numeric() || *c == '.')
                .collect::<String>()
                .parse::<f32>()
                .ok()
        });

        // 리뷰 수 추출
        let review_selectors = vec![
            ".review-count",
            ".rating-num",
            "[class*='review']",
            "[class*='sold']",
        ];
        let review_text = self.try_extract_text(&element, &review_selectors);
        let review_count = review_text.and_then(|r| {
            r.chars()
                .filter(|c| c.is_numeric())
                .collect::<String>()
                .parse::<u32>()
                .ok()
        });

        // 이미지 추출
        let image_selectors = vec!["img"];
        let image_url = self.try_extract_image(&element, &image_selectors);

        Some(Product {
            name,
            price,
            original_price: None,
            discount_rate: None,
            rating,
            review_count,
            image_url,
            product_url: utils::make_absolute_url("https://www.aliexpress.com", &product_url),
            seller: None,
            delivery_info: None,
            source: ProductSource::AliExpress,
            raw_data: None,
        })
    }

    fn try_extract_text(&self, element: &ElementRef, selectors: &[&str]) -> Option<String> {
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(elem) = element.select(&selector).next() {
                    let text = elem.text().collect::<String>().trim().to_string();
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
        }
        None
    }

    fn try_extract_attr(&self, element: &ElementRef, selectors: &[&str], attr: &str) -> Option<String> {
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(elem) = element.select(&selector).next() {
                    if let Some(value) = elem.value().attr(attr) {
                        if !value.is_empty() {
                            return Some(value.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn try_extract_image(&self, element: &ElementRef, selectors: &[&str]) -> Option<String> {
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(elem) = element.select(&selector).next() {
                    if let Some(src) = elem
                        .value()
                        .attr("src")
                        .or_else(|| elem.value().attr("data-src"))
                        .or_else(|| elem.value().attr("data-image"))
                    {
                        return Some(utils::make_absolute_url(
                            "https://www.aliexpress.com",
                            src,
                        ));
                    }
                }
            }
        }
        None
    }

    fn parse_products(&self, html: &str) -> Result<Vec<Product>> {
        let document = Html::parse_document(html);
        let mut products = Vec::new();

        // 여러 가능한 상품 컨테이너 셀렉터 시도
        let container_selectors = vec![
            "div.list-item",
            "div.product-item",
            "a.item-anchor",
            "[class*='product-item']",
            "[class*='list-item']",
            "div[data-product-id]",
        ];

        for container_selector in container_selectors {
            if let Ok(selector) = Selector::parse(container_selector) {
                let elements: Vec<_> = document.select(&selector).collect();

                if !elements.is_empty() {
                    debug!(
                        "Found {} product containers with selector: {}",
                        elements.len(),
                        container_selector
                    );

                    for element in elements {
                        if let Some(product) = self.extract_product_from_element(element) {
                            products.push(product);
                        }
                    }

                    if !products.is_empty() {
                        break;
                    }
                }
            }
        }

        // AliExpress는 자바스크립트로 렌더링되는 경우가 많아 파싱이 어려울 수 있음
        if products.is_empty() {
            warn!("No products found with standard selectors. AliExpress may be using JS rendering.");
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
        info!(
            "Starting AliExpress crawl for query: {}",
            config.search_query
        );
        let mut all_products = Vec::new();

        for page in 1..=config.max_pages {
            debug!("Crawling AliExpress page {}", page);

            let url = Self::build_search_url(&config.search_query, page);

            match fetch_with_retry(&self.client, &url, 3).await {
                Ok(html) => match self.parse_products(&html) {
                    Ok(mut products) => {
                        info!("Found {} products on page {}", products.len(), page);

                        if products.is_empty() && page > 1 {
                            info!("No more products found, stopping crawl");
                            break;
                        }

                        all_products.append(&mut products);
                    }
                    Err(e) => {
                        warn!("Failed to parse products on page {}: {}", page, e);
                    }
                },
                Err(e) => {
                    warn!("Failed to fetch page {}: {}", page, e);
                }
            }

            // AliExpress는 더 긴 딜레이 권장 (봇 감지 방지)
            tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;
        }

        info!(
            "AliExpress crawl completed. Total products: {}",
            all_products.len()
        );
        Ok(all_products)
    }

    async fn extract_product(&self, url: &str) -> Result<Option<Product>> {
        debug!("Extracting product from URL: {}", url);

        let html = fetch_with_retry(&self.client, url, 3).await?;
        let document = Html::parse_document(&html);

        let name_selectors = vec![
            "h1.product-title-text",
            "h1",
            ".product-title",
            "[class*='product-title']",
        ];
        let name = utils::try_selectors_text(&document, &name_selectors);

        if name.is_none() || name.as_ref().unwrap().is_empty() {
            return Ok(None);
        }

        let price_selectors = vec![
            ".product-price-value",
            ".price",
            "[class*='price']",
        ];
        let price = utils::try_selectors_text(&document, &price_selectors);

        Ok(Some(Product {
            name: name.unwrap(),
            price,
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
