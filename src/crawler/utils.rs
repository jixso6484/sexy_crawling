use scraper::{Html, Selector};

/// 여러 셀렉터를 시도하여 첫 번째 매칭되는 요소의 텍스트 반환
pub fn try_selectors_text(document: &Html, selectors: &[&str]) -> Option<String> {
    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element.text().collect::<String>().trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    None
}

/// 여러 셀렉터를 시도하여 속성 값 반환
pub fn try_selectors_attr<'a>(
    document: &'a Html,
    selectors: &[&str],
    attr: &str,
) -> Option<String> {
    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                if let Some(value) = element.value().attr(attr) {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// 가격 문자열에서 숫자만 추출
pub fn extract_numeric_price(price_str: &str) -> Option<f64> {
    let cleaned: String = price_str
        .chars()
        .filter(|c| c.is_numeric() || *c == '.')
        .collect();
    cleaned.parse::<f64>().ok()
}

/// URL을 절대 경로로 변환
pub fn make_absolute_url(base: &str, url: &str) -> String {
    if url.starts_with("http") {
        url.to_string()
    } else if url.starts_with("//") {
        format!("https:{}", url)
    } else if url.starts_with('/') {
        // base URL에서 도메인만 추출 (https:// 이후 첫 번째 / 찾기)
        if base.len() > 8 {
            if let Some(domain_end) = base[8..].find('/') {
                format!("{}{}", &base[..8 + domain_end], url)
            } else {
                format!("{}{}", base, url)
            }
        } else {
            format!("{}{}", base, url)
        }
    } else {
        format!("{}/{}", base, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_numeric_price() {
        assert_eq!(extract_numeric_price("1,234,567원"), Some(1234567.0));
        assert_eq!(extract_numeric_price("$99.99"), Some(99.99));
        assert_eq!(extract_numeric_price("무료"), None);
    }

    #[test]
    fn test_make_absolute_url() {
        assert_eq!(
            make_absolute_url("https://example.com", "https://other.com/path"),
            "https://other.com/path"
        );
        assert_eq!(
            make_absolute_url("https://example.com", "//cdn.example.com/img.jpg"),
            "https://cdn.example.com/img.jpg"
        );
        assert_eq!(
            make_absolute_url("https://example.com/page", "/product/123"),
            "https://example.com/product/123"
        );
    }
}
