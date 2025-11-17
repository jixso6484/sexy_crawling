use async_trait::async_trait;
use scraper::{ElementRef, Html, Selector};
use std::path::PathBuf;
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

    fn build_category_url(category_url: &str, page: u32) -> String {
        // 카테고리 URL에 페이지 파라미터 추가
        if category_url.contains('?') {
            // 이미 쿼리 파라미터가 있는 경우
            if category_url.contains("&page=") || category_url.contains("?page=") {
                // 기존 page 파라미터 교체
                let re = regex::Regex::new(r"[?&]page=\d+").unwrap();
                re.replace(category_url, &format!("&page={}", page)).to_string()
            } else {
                format!("{}&page={}", category_url, page)
            }
        } else {
            format!("{}?page={}", category_url, page)
        }
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
        let crawl_mode = if config.category_url.is_some() {
            "category"
        } else {
            "search"
        };

        info!(
            "Starting Danawa {} crawl for: {}",
            crawl_mode,
            config.category_url.as_deref().unwrap_or(&config.search_query)
        );

        // 출력 디렉토리 설정
        let save_to_disk = config.output_dir.is_some();
        if let Some(output_dir) = &config.output_dir {
            let output_path = PathBuf::from(output_dir);

            // 디렉토리 생성
            std::fs::create_dir_all(&output_path)?;
            std::fs::create_dir_all(output_path.join("html"))?;
            std::fs::create_dir_all(output_path.join("products"))?;

            info!("Saving results to: {:?}", output_path);
        }

        let mut all_products = Vec::new();
        let mut page = 1;
        let mut consecutive_empty_pages = 0;
        let max_consecutive_empty = 2; // 연속으로 빈 페이지 2개 나오면 중단
        let mut visited_urls = Vec::new();

        loop {
            // max_pages 체크 (crawl_all_pages가 false일 때만)
            if !config.crawl_all_pages && page > config.max_pages {
                info!("Reached max_pages limit ({})", config.max_pages);
                break;
            }

            debug!("Crawling Danawa page {}", page);

            let url = if let Some(category_url) = &config.category_url {
                Self::build_category_url(category_url, page)
            } else {
                Self::build_search_url(&config.search_query, page)
            };

            visited_urls.push(url.clone());

            match fetch_with_retry(&self.client, &url, 3).await {
                Ok(html) => {
                    // HTML 저장 (옵션)
                    if save_to_disk {
                        if let Some(output_dir) = &config.output_dir {
                            let html_file = PathBuf::from(output_dir)
                                .join("html")
                                .join(format!("page_{:04}.html", page));

                            if let Err(e) = std::fs::write(&html_file, &html) {
                                warn!("Failed to save HTML for page {}: {}", page, e);
                            } else {
                                debug!("Saved HTML to {:?}", html_file);
                            }
                        }
                    }

                    match self.parse_products(&html) {
                        Ok(mut products) => {
                            info!("Found {} products on page {}", products.len(), page);

                            if products.is_empty() {
                                consecutive_empty_pages += 1;
                                if consecutive_empty_pages >= max_consecutive_empty {
                                    info!(
                                        "No products found for {} consecutive pages, stopping crawl",
                                        max_consecutive_empty
                                    );
                                    break;
                                }
                            } else {
                                consecutive_empty_pages = 0; // 상품이 있으면 카운터 리셋

                                // 상품 정보를 즉시 파일에 저장 (append 모드)
                                if save_to_disk {
                                    if let Some(output_dir) = &config.output_dir {
                                        let products_file = PathBuf::from(output_dir)
                                            .join("products")
                                            .join(format!("page_{:04}.json", page));

                                        match serde_json::to_string_pretty(&products) {
                                            Ok(json) => {
                                                if let Err(e) = std::fs::write(&products_file, json) {
                                                    warn!("Failed to save products for page {}: {}", page, e);
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Failed to serialize products for page {}: {}", page, e);
                                            }
                                        }
                                    }
                                }

                                all_products.append(&mut products);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse products on page {}: {}", page, e);
                            consecutive_empty_pages += 1;
                            if consecutive_empty_pages >= max_consecutive_empty {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch page {}: {}", page, e);
                    consecutive_empty_pages += 1;
                    if consecutive_empty_pages >= max_consecutive_empty {
                        break;
                    }
                }
            }

            page += 1;

            // 너무 많은 페이지 크롤링 방지 (최대 100페이지)
            if page > 100 {
                warn!("Reached maximum page limit (100), stopping crawl");
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
        }

        // 방문한 URL 목록 저장
        if save_to_disk {
            if let Some(output_dir) = &config.output_dir {
                let visited_file = PathBuf::from(output_dir).join("visited_urls.json");
                let visited_data = serde_json::json!({
                    "total_pages": page - 1,
                    "urls": visited_urls
                });

                if let Ok(json) = serde_json::to_string_pretty(&visited_data) {
                    let _ = std::fs::write(visited_file, json);
                }
            }
        }

        info!(
            "Danawa crawl completed. Total products: {} from {} pages",
            all_products.len(),
            page - 1
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
