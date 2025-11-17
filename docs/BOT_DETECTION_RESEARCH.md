# 봇 탐지 회피 연구 및 구현 가이드

## 🔍 봇 탐지 시스템이 확인하는 항목들

### 1. **HTTP 헤더 분석**
- ✅ User-Agent 일관성
- ✅ Accept, Accept-Language, Accept-Encoding 순서
- ✅ Sec-Fetch-* 헤더 (Chromium 기반)
- ⚠️ 헤더 순서 (HTTP/2 헤더 우선순위)
- ⚠️ 헤더 대소문자 (일부 봇은 소문자만 사용)

### 2. **TLS 핑거프린트**
```
실제 브라우저:
- TLS 버전: TLS 1.3
- Cipher Suites: 브라우저별 고유 순서
- Extensions: ALPN, SNI, session_ticket 등
- Curves: x25519, secp256r1, secp384r1

봇의 특징:
- Go HTTP 클라이언트: 특정 cipher suite 순서
- Python requests: TLS 1.2 기본
- Rust reqwest: OpenSSL/BoringSSL 기본값
```

**해결책:**
- Reqwest는 기본적으로 BoringSSL 사용 → 괜찮음
- 하지만 완벽한 우회는 `curl_impersonate` 필요

### 3. **JavaScript 실행 여부**
```javascript
// 봇 탐지 코드 예시
if (!window.navigator.webdriver) {
  // 정상 브라우저
} else {
  // Selenium/WebDriver 감지
}

// Canvas 핑거프린트
const canvas = document.createElement('canvas');
const ctx = canvas.getContext('2d');
// ... 그리기 후 해시 생성
```

**우리의 한계:**
- HTTP 클라이언트는 JavaScript 실행 불가
- Headless Browser 필요 (나중에 추가)

### 4. **요청 패턴 분석**
```
정상 사용자:
- 불규칙한 요청 간격 (1-10초)
- 이미지, CSS, JS 파일도 요청
- Referer 체인이 자연스러움
- 세션 지속 시간: 수분~수십분

봇의 특징:
- 규칙적인 요청 간격 (정확히 1초마다)
- HTML만 요청
- Referer가 없거나 부자연스러움
- 세션 지속 시간: 수초
```

### 5. **쿠키 및 세션 관리**
```
정상 브라우저:
- 쿠키 자동 저장 및 전송
- 세션 유지 (여러 페이지 방문)
- 로컬 스토리지 사용

봇의 특징:
- 쿠키 무시
- 매번 새 세션
- 상태 관리 없음
```

### 6. **화면 해상도 및 환경**
```javascript
// 브라우저 환경 정보
screen.width, screen.height
window.innerWidth, window.innerHeight
navigator.platform
navigator.language
navigator.hardwareConcurrency
```

**우리의 한계:**
- HTTP 클라이언트는 이 정보 제공 불가
- Headless Browser에서만 가능

---

## ✅ 현재 구현된 회피 기술

1. **User-Agent 로테이션** (14개)
2. **실제 브라우저 헤더** (Accept, Accept-Language, etc.)
3. **Sec-Fetch-* 헤더** (Chromium)
4. **Referer 체인**
5. **쿠키 저장소** (reqwest 기본 기능)
6. **랜덤 딜레이** (0.5-2초)
7. **사람 행동 시뮬레이션** (페이지 읽기, 스크롤, 휴식)
8. **Rate Limiting** (도메인별)

---

## 🚀 추가 구현 항목

### Priority 1: 즉시 구현 가능
- [ ] 더 많은 User-Agent (50+)
- [ ] 브라우저 프로필 (Chrome, Firefox, Safari 세트)
- [ ] 헤더 순서 일관성 유지
- [ ] 세션 쿠키 자동 관리
- [ ] IP 로테이션 (프록시)
- [ ] 더 정교한 타이밍 (가우시안 분포)

### Priority 2: 중급 난이도
- [ ] TLS 핑거프린트 커스터마이징
- [ ] HTTP/2 설정 조정
- [ ] 이미지/CSS/JS 요청 시뮬레이션
- [ ] 자연스러운 네비게이션 경로

### Priority 3: 고급 (Headless Browser 필요)
- [ ] JavaScript 실행 환경
- [ ] Canvas 핑거프린트 생성
- [ ] WebGL 핑거프린트
- [ ] 마우스 이벤트 시뮬레이션
- [ ] 화면 스크롤 이벤트

---

## 📊 봇 탐지 서비스 비교

### Cloudflare
- **탐지 방법:** TLS 핑거프린트, JavaScript Challenge, CAPTCHA
- **우회 난이도:** 높음
- **대응책:** curl_impersonate, Headless Browser

### DataDome
- **탐지 방법:** 행동 패턴, 마우스 움직임, JavaScript
- **우회 난이도:** 매우 높음
- **대응책:** 완벽한 브라우저 시뮬레이션

### PerimeterX
- **탐지 방법:** 다양한 센서, ML 기반
- **우회 난이도:** 매우 높음
- **대응책:** 실제 브라우저 사용

### 알리익스프레스
- **탐지 방법:** User-Agent, 요청 빈도, IP
- **우회 난이도:** 중간
- **대응책:** 현재 구현 + 프록시 로테이션

---

## 🎯 권장 전략

### 단계별 접근
1. **Phase 1 (현재):** HTTP 클라이언트 + 고급 헤더
2. **Phase 2:** 프록시 로테이션 추가
3. **Phase 3:** Headless Browser (선택적)
4. **Phase 4:** 완전 자동화

### 사이트별 전략
| 사이트 | 난이도 | 전략 |
|--------|--------|------|
| 알리익스프레스 | 중간 | HTTP + 프록시 |
| 쿠팡 | 낮음 | HTTP 클라이언트 |
| 아마존 | 높음 | Headless Browser |
| 구글 | 매우 높음 | 실제 브라우저 + IP 로테이션 |

---

## 🔧 구현 예정 개선사항

```rust
// 1. 브라우저 프로필 시스템
struct BrowserProfile {
    user_agent: String,
    headers: HeaderMap,
    tls_config: TlsConfig,
    cookies: CookieStore,
}

// 2. 세션 관리
struct CrawlSession {
    profile: BrowserProfile,
    visited_urls: Vec<String>,
    cookies: HashMap<String, String>,
    start_time: Instant,
}

// 3. 지능형 딜레이
fn intelligent_delay() -> Duration {
    // 가우시안 분포 사용
    normal_distribution(mean=2.0, std=0.5)
}
```

---

## 📚 참고 자료

- [TLS Fingerprinting](https://tlsfingerprint.io/)
- [Browser Fingerprinting](https://fingerprintjs.com/)
- [Cloudflare Bot Management](https://www.cloudflare.com/bot-management/)
- [curl-impersonate](https://github.com/lwthiker/curl-impersonate)
- [undetected-chromedriver](https://github.com/ultrafunkamsterdam/undetected-chromedriver)

---

## 🎓 학습 내용

### 왜 봇이 탐지되는가?
1. **일관성 부족:** User-Agent는 Chrome인데 헤더는 Firefox
2. **패턴 인식:** 정확히 1초마다 요청
3. **핑거프린트 불일치:** TLS 핑거프린트가 실제 브라우저와 다름
4. **JavaScript 없음:** Headless 감지

### 어떻게 우회하는가?
1. **완벽한 일관성:** 모든 헤더가 실제 브라우저와 동일
2. **무작위성:** 요청 간격, 순서, 타이밍 모두 랜덤
3. **세션 유지:** 쿠키, Referer 체인 관리
4. **환경 시뮬레이션:** 실제 브라우저처럼 동작

### 한계는 무엇인가?
1. **JavaScript:** HTTP 클라이언트는 실행 불가
2. **TLS:** 완벽한 우회는 어려움 (curl_impersonate 필요)
3. **행동 패턴:** ML 기반 탐지는 매우 어려움
4. **비용:** 프록시, Headless Browser는 리소스 소모

---

**결론:** 현재 구현은 중간 수준의 봇 탐지를 우회할 수 있습니다.
더 강력한 우회가 필요하면 Headless Browser와 프록시를 추가해야 합니다.
