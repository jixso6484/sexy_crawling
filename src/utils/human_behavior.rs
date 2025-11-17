use rand::Rng;
use std::time::Duration;
use tracing::debug;

/// 사람처럼 행동하는 크롤링 시뮬레이터
#[derive(Debug, Clone, serde::Serialize)]
pub struct HumanBehavior {
    pub page_reading_time: (u64, u64),      // 페이지 읽기 시간 (초) - (최소, 최대)
    pub scroll_pause_time: (u64, u64),       // 스크롤 사이 멈춤 시간 (밀리초)
    pub click_delay: (u64, u64),             // 클릭 딜레이 (밀리초)
    pub typing_speed: (u64, u64),            // 타이핑 속도 (글자당 밀리초)
    pub session_duration: (u64, u64),        // 세션 지속 시간 (분)
    pub break_probability: f64,              // 휴식 확률 (0.0 - 1.0)
    pub break_duration: (u64, u64),          // 휴식 시간 (초)
}

impl Default for HumanBehavior {
    fn default() -> Self {
        Self {
            page_reading_time: (2, 8),       // 2-8초 페이지 읽기
            scroll_pause_time: (500, 1500),  // 0.5-1.5초 스크롤 멈춤
            click_delay: (200, 800),         // 0.2-0.8초 클릭 딜레이
            typing_speed: (100, 300),        // 글자당 0.1-0.3초
            session_duration: (5, 15),       // 5-15분 세션
            break_probability: 0.05,         // 5% 확률로 휴식
            break_duration: (30, 120),       // 30초-2분 휴식
        }
    }
}

impl HumanBehavior {
    /// 더 빠른 크롤링 (덜 안전)
    pub fn fast() -> Self {
        Self {
            page_reading_time: (1, 3),
            scroll_pause_time: (200, 500),
            click_delay: (100, 300),
            typing_speed: (50, 150),
            session_duration: (3, 8),
            break_probability: 0.02,
            break_duration: (10, 30),
        }
    }

    /// 더 느린 크롤링 (더 안전)
    pub fn slow() -> Self {
        Self {
            page_reading_time: (5, 15),
            scroll_pause_time: (1000, 3000),
            click_delay: (500, 1500),
            typing_speed: (150, 400),
            session_duration: (10, 30),
            break_probability: 0.1,
            break_duration: (60, 180),
        }
    }

    /// 랜덤 값 생성
    fn random_in_range(&self, range: (u64, u64)) -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(range.0..=range.1)
    }

    /// 페이지 읽기 시간 (사람은 페이지를 읽는 데 시간이 걸림)
    pub async fn simulate_page_reading(&self) {
        let duration = self.random_in_range(self.page_reading_time);
        debug!("Simulating page reading for {} seconds", duration);
        tokio::time::sleep(Duration::from_secs(duration)).await;
    }

    /// 스크롤 시뮬레이션 (여러 번 스크롤하며 멈춤)
    pub async fn simulate_scrolling(&self, scroll_count: u32) {
        debug!("Simulating {} scroll actions", scroll_count);

        for i in 0..scroll_count {
            let pause = self.random_in_range(self.scroll_pause_time);
            debug!("Scroll {} - pausing for {}ms", i + 1, pause);
            tokio::time::sleep(Duration::from_millis(pause)).await;
        }
    }

    /// 클릭 딜레이 (사람은 즉시 클릭하지 않음)
    pub async fn simulate_click_delay(&self) {
        let delay = self.random_in_range(self.click_delay);
        debug!("Simulating click delay: {}ms", delay);
        tokio::time::sleep(Duration::from_millis(delay)).await;
    }

    /// 타이핑 시뮬레이션 (검색어 입력)
    pub async fn simulate_typing(&self, text_length: usize) {
        let total_time: u64 = (0..text_length)
            .map(|_| self.random_in_range(self.typing_speed))
            .sum();

        debug!("Simulating typing {} characters for {}ms", text_length, total_time);
        tokio::time::sleep(Duration::from_millis(total_time)).await;
    }

    /// 랜덤 휴식 (가끔 사람은 멈춤)
    pub async fn maybe_take_break(&self) -> bool {
        let mut rng = rand::thread_rng();

        if rng.gen_bool(self.break_probability) {
            let break_time = self.random_in_range(self.break_duration);
            debug!("Taking a random break for {} seconds", break_time);
            tokio::time::sleep(Duration::from_secs(break_time)).await;
            true
        } else {
            false
        }
    }

    /// 페이지 간 네비게이션 딜레이
    pub async fn simulate_navigation(&self) {
        // 클릭 후 페이지 로딩 대기
        self.simulate_click_delay().await;

        // 새 페이지 로딩 시간 (네트워크 + 렌더링)
        let loading_time = self.random_in_range((500, 2000));
        debug!("Simulating page loading: {}ms", loading_time);
        tokio::time::sleep(Duration::from_millis(loading_time)).await;
    }

    /// 전체 방문 시뮬레이션 (페이지 읽기 + 스크롤)
    pub async fn simulate_page_visit(&self) {
        // 페이지 로딩 후 스크롤
        let scroll_count = rand::thread_rng().gen_range(2..=5);
        self.simulate_scrolling(scroll_count).await;

        // 페이지 읽기
        self.simulate_page_reading().await;

        // 가끔 휴식
        self.maybe_take_break().await;
    }
}

/// 검색어 변형 (사람은 다양한 검색어를 사용)
pub struct QueryVariator {
    synonyms: Vec<Vec<String>>,  // 동의어 그룹
}

impl QueryVariator {
    pub fn new() -> Self {
        Self {
            synonyms: vec![
                vec!["게이밍".to_string(), "gaming".to_string(), "gamer".to_string()],
                vec!["마우스".to_string(), "mouse".to_string()],
                vec!["키보드".to_string(), "keyboard".to_string()],
                vec!["헤드셋".to_string(), "headset".to_string(), "headphone".to_string()],
                vec!["저렴한".to_string(), "cheap".to_string(), "budget".to_string(), "affordable".to_string()],
                vec!["고급".to_string(), "premium".to_string(), "high-end".to_string()],
            ],
        }
    }

    /// 검색어 변형 생성
    pub fn create_variations(&self, query: &str) -> Vec<String> {
        let mut variations = vec![query.to_string()];

        // 기본 변형들
        variations.push(format!("{} 추천", query));
        variations.push(format!("{} best", query));
        variations.push(format!("best {}", query));
        variations.push(format!("{} 인기", query));
        variations.push(format!("{} popular", query));

        // 필터 추가
        variations.push(format!("{} 세일", query));
        variations.push(format!("{} sale", query));
        variations.push(format!("{} discount", query));

        // 중복 제거
        variations.sort();
        variations.dedup();

        variations
    }
}

/// 탐색 패턴 (사람은 순차적으로만 탐색하지 않음)
#[derive(Debug, Clone)]
pub enum BrowsingPattern {
    Sequential,      // 순차적 (1, 2, 3, 4, ...)
    Random,          // 랜덤 (3, 1, 7, 2, ...)
    MixedRandom,     // 혼합 (1, 2, 5, 3, 8, 4, ...)
    Backtracking,    // 뒤로가기 포함 (1, 2, 1, 3, 2, 4, ...)
}

impl BrowsingPattern {
    /// 페이지 순서 생성
    pub fn generate_page_order(&self, total_pages: u32) -> Vec<u32> {
        let mut pages: Vec<u32> = (1..=total_pages).collect();

        match self {
            BrowsingPattern::Sequential => pages,
            BrowsingPattern::Random => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                pages.shuffle(&mut rng);
                pages
            }
            BrowsingPattern::MixedRandom => {
                let mut result = Vec::new();
                let mut remaining = pages.clone();
                let mut rng = rand::thread_rng();

                while !remaining.is_empty() {
                    // 70% 확률로 순차적, 30% 확률로 랜덤
                    if rng.gen_bool(0.7) && !result.is_empty() {
                        // 다음 페이지
                        let last = result.last().unwrap();
                        if let Some(idx) = remaining.iter().position(|&x| x == last + 1) {
                            result.push(remaining.remove(idx));
                            continue;
                        }
                    }

                    // 랜덤 페이지
                    let idx = rng.gen_range(0..remaining.len());
                    result.push(remaining.remove(idx));
                }

                result
            }
            BrowsingPattern::Backtracking => {
                let mut result = Vec::new();
                let mut rng = rand::thread_rng();

                for page in pages {
                    result.push(page);

                    // 20% 확률로 이전 페이지 재방문
                    if page > 1 && rng.gen_bool(0.2) {
                        let prev_page = rng.gen_range(1..page);
                        result.push(prev_page);
                    }
                }

                result
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browsing_patterns() {
        let pattern = BrowsingPattern::Sequential;
        let order = pattern.generate_page_order(5);
        assert_eq!(order, vec![1, 2, 3, 4, 5]);

        let pattern = BrowsingPattern::Random;
        let order = pattern.generate_page_order(5);
        assert_eq!(order.len(), 5);
        assert!(order.contains(&1));
        assert!(order.contains(&5));
    }

    #[test]
    fn test_query_variations() {
        let variator = QueryVariator::new();
        let variations = variator.create_variations("gaming mouse");

        assert!(variations.len() > 1);
        assert!(variations.contains(&"gaming mouse".to_string()));
        assert!(variations.iter().any(|v| v.contains("best")));
    }

    #[tokio::test]
    async fn test_human_behavior() {
        let behavior = HumanBehavior::fast();

        let start = tokio::time::Instant::now();
        behavior.simulate_click_delay().await;
        let elapsed = start.elapsed();

        // 0.1초 ~ 0.3초 사이여야 함
        assert!(elapsed.as_millis() >= 100);
        assert!(elapsed.as_millis() <= 400);  // 여유 있게
    }
}
