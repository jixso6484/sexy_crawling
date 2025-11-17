use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, info, warn};

use crate::crawler::{create_http_client, fetch_with_retry, utils, Crawler};
use crate::error::Result;
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
        let page_num = (page - 1) * 30;
        format!(
            "https://search.danawa.com/dsearch.php?query={}&page={}&limit=30",
            urlencoding::encode(query),
            page_num
        )
    }

    fn extract_product_from_element(&self, element: ElementRef) -> Option<Product> {
        // 상품명 및 링크 추출
        let name_link_selectors = vec![
            ".prod_name a",
            ".prod_info a",
            "a.prod_name",
            "a.product-name",
            "[class*='prod_name'] a",
        ];

        let (name, product_url) = self.try_extract_name_and_link(&element, &name_link_selectors)?;

        if name.is_empty() || product_url.is_empty() {
            return None;
        }

        // 가격 추출 - 여러 셀렉터 시도
        let price_selectors = vec![
            ".price_sect strong",
            ".price em",
            ".prod_pric",
            "strong.price",
            "[class*='price'] strong",
            "[class*='price'] em",
        ];
        let price = self.try_extract_text(&element, &price_selectors);

        // 이미지 추출
        let image_selectors = vec![
            ".thumb_image img",
            ".prod_img img",
            "img.thumb",
            "[class*='thumb'] img",
            "img[class*='prod']",
        ];
        let image_url = self.try_extract_image(&element, &image_selectors);

        // 판매처 추출
        let seller_selectors = vec![
            ".mall_name",
            ".seller",
            "[class*='mall']",
            "[class*='seller']",
        ];
        let seller = self.try_extract_text(&element, &seller_selectors);

        // 배송 정보
        let delivery_selectors = vec![
            ".delivery",
            ".shipping",
            "[class*='delivery']",
            "[class*='shipping']",
        ];
        let delivery_info = self.try_extract_text(&element, &delivery_selectors);

        Some(Product {
            name,
            price,
            original_price: None,
            discount_rate: None,
            rating: None,
            review_count: None,
            image_url,
            product_url: utils::make_absolute_url("https://search.danawa.com", &product_url),
            seller,
            delivery_info,
            source: ProductSource::Danawa,
            raw_data: None,
        })
    }

    fn try_extract_name_and_link(
        &self,
        element: &ElementRef,
        selectors: &[&str],
    ) -> Option<(String, String)> {
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(elem) = element.select(&selector).next() {
                    let name = elem.text().collect::<String>().trim().to_string();
                    if let Some(href) = elem.value().attr("href") {
                        if !name.is_empty() && !href.is_empty() {
                            return Some((name, href.to_string()));
                        }
                    }
                }
            }
        }
        None
    }

    fn try_extract_text(&self, element: &ElementRef, selectors: &[&str]) -> Option<String> {
        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(elem) = element.select(&selector).next() {
                    let text = elem
                        .text()
                        .collect::<String>()
                        .trim()
                        .replace(",", "")
                        .replace("원", "")
                        .trim()
                        .to_string();
                    if !text.is_empty() {
                        return Some(text);
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
                        .or_else(|| elem.value().attr("data-original"))
                        .or_else(|| elem.value().attr("data-src"))
                    {
                        return Some(utils::make_absolute_url("https://search.danawa.com", src));
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
            "div.prod_item",
            "li.prod_item",
            ".product_list .prod_item",
            "[class*='prod_item']",
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

            tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
        }

        info!(
            "Danawa crawl completed. Total products: {}",
            all_products.len()
        );
        Ok(all_products)
    }

    async fn extract_product(&self, url: &str) -> Result<Option<Product>> {
        debug!("Extracting product from URL: {}", url);

        let html = fetch_with_retry(&self.client, url, 3).await?;
        let document = Html::parse_document(&html);

        let name_selectors = vec![".prod_tit", ".top_summary h3", "h1.prod_title", "h1"];
        let name = utils::try_selectors_text(&document, &name_selectors);

        if name.is_none() || name.as_ref().unwrap().is_empty() {
            return Ok(None);
        }

        let price_selectors = vec![".lowest_price", ".price_sect strong", ".sale-price"];
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
            source: ProductSource::Danawa,
            raw_data: None,
        }))
    }
}
