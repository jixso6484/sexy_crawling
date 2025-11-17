use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, info, warn};

use crate::crawler::{create_http_client, fetch_with_retry, utils, Crawler};
use crate::error::Result;
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
        format!(
            "https://www.coupang.com/np/search?q={}&page={}",
            urlencoding::encode(query),
            page
        )
    }

    fn extract_product_from_element(&self, element: ElementRef) -> Option<Product> {
        // 상품명 추출 - 여러 셀렉터 시도
        let name_selectors = vec![
            ".name",
            ".prod-name",
            ".product-name",
            "[class*='name']",
            "dt.name",
            "a.name",
        ];

        let name = self.try_extract_text(&element, &name_selectors)?;
        if name.is_empty() {
            return None;
        }

        // 링크 추출
        let link_selectors = vec![
            "a.search-product-link",
            "a[href*='/products/']",
            "a.prod-link",
            ".search-product a",
        ];
        let product_url = self.try_extract_link(&element, &link_selectors)?;
        if product_url.is_empty() {
            return None;
        }

        // 가격 추출
        let price_selectors = vec![
            ".price-value",
            "strong.price-value",
            ".actual-price",
            ".sale-price",
            "[class*='price']",
        ];
        let price = self.try_extract_text(&element, &price_selectors);

        // 할인율 추출
        let discount_selectors = vec![
            ".discount-percentage",
            ".discount-rate",
            "[class*='discount']",
        ];
        let discount_rate = self.try_extract_text(&element, &discount_selectors);

        // 평점 추출
        let rating_selectors = vec![".rating", ".star-rating", "[class*='rating']"];
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
            ".rating-total-count",
            ".review-count",
            "[class*='review']",
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
        let image_selectors = vec![
            "img.search-product-wrap-img",
            "img.product-image",
            "img[class*='product']",
            "img",
        ];
        let image_url = self.try_extract_image(&element, &image_selectors);

        // 배송 정보 추출
        let delivery_selectors = vec![
            ".shipping",
            ".delivery-info",
            "[class*='delivery']",
            "[class*='shipping']",
        ];
        let delivery_info = self.try_extract_text(&element, &delivery_selectors);

        Some(Product {
            name,
            price,
            original_price: None,
            discount_rate,
            rating,
            review_count,
            image_url,
            product_url: utils::make_absolute_url("https://www.coupang.com", &product_url),
            seller: None,
            delivery_info,
            source: ProductSource::Coupang,
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

    fn try_extract_link(&self, element: &ElementRef, selectors: &[&str]) -> Option<String> {
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(elem) = element.select(&selector).next() {
                    if let Some(href) = elem.value().attr("href") {
                        return Some(href.to_string());
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
                        .or_else(|| elem.value().attr("data-img-src"))
                        .or_else(|| elem.value().attr("data-src"))
                    {
                        return Some(utils::make_absolute_url("https://www.coupang.com", src));
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
            "li.search-product",
            "ul.search-product-list > li",
            "[class*='search-product']",
            "li[id*='product']",
            ".product-item",
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

                    // 상품을 찾았으면 다른 셀렉터는 시도하지 않음
                    if !products.is_empty() {
                        break;
                    }
                }
            }
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

            // 재시도 로직을 포함한 요청
            match fetch_with_retry(&self.client, &url, 3).await {
                Ok(html) => match self.parse_products(&html) {
                    Ok(mut products) => {
                        info!("Found {} products on page {}", products.len(), page);

                        // 데이터가 없으면 다음 페이지도 없을 가능성이 높음
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

            // 요청 간 딜레이 (서버 부하 방지)
            tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
        }

        info!(
            "Coupang crawl completed. Total products: {}",
            all_products.len()
        );
        Ok(all_products)
    }

    async fn extract_product(&self, url: &str) -> Result<Option<Product>> {
        debug!("Extracting product from URL: {}", url);

        let html = fetch_with_retry(&self.client, url, 3).await?;
        let document = Html::parse_document(&html);

        // 상세 페이지에서 더 많은 정보 추출
        let name_selectors = vec![
            "h2.prod-buy-header__title",
            ".product-title",
            "h1.prod-title",
            "h1",
        ];
        let name = utils::try_selectors_text(&document, &name_selectors);

        if name.is_none() || name.as_ref().unwrap().is_empty() {
            return Ok(None);
        }

        let price_selectors = vec![
            ".total-price strong",
            ".sale-price",
            ".price-value",
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
            source: ProductSource::Coupang,
            raw_data: None,
        }))
    }
}
