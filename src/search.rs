use std::time::Duration;

use crate::config::WebSearchConfig;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// DDG HTML search (no API key required).
async fn search_ddg(query: &str, config: &WebSearchConfig) -> anyhow::Result<Vec<SearchResult>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_secs))
        .build()?;

    let url = format!("https://html.duckduckgo.com/html/?q={}", url_escape(query));
    let resp = client
        .get(&url)
        .header("User-Agent", "mimo-opt/0.5 (terminal-ai-tool)")
        .send()
        .await?;

    let body = resp.text().await?;
    let results = parse_ddg_html(&body, config.max_results);
    Ok(results)
}

fn url_escape(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

fn parse_ddg_html(html: &str, max_results: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let mut search_from = 0usize;

    for _ in 0..max_results {
        let body_start = match html[search_from..].find("class=\"result__body\"") {
            Some(pos) => search_from + pos,
            None => break,
        };
        let block = &html[body_start..];

        // Extract title and URL from <a class="result__a" href="...">...</a>
        let (title, url) = if let Some(link_start) = block.find("class=\"result__a\"") {
            let after_link = &block[link_start..];
            let href = if let Some(href_start) = after_link.find("href=\"") {
                let href_content = &after_link[href_start + 6..];
                if let Some(href_end) = href_content.find('"') {
                    href_content[..href_end].to_string()
                } else {
                    break;
                }
            } else {
                break;
            };

            let title_text = if let Some(tag_end) = after_link.find('>') {
                let after_tag = &after_link[tag_end + 1..];
                if let Some(close_tag) = after_tag.find("</a>") {
                    strip_html(&after_tag[..close_tag])
                } else {
                    break;
                }
            } else {
                break;
            };

            (title_text, clean_ddg_url(&href))
        } else {
            break;
        };

        // Extract snippet from <a class="result__snippet"
        let snippet = if let Some(snip_start) = block.find("class=\"result__snippet\"") {
            let after_snip = &block[snip_start..];
            if let Some(tag_end) = after_snip.find('>') {
                let after_tag = &after_snip[tag_end + 1..];
                if let Some(close_tag) = after_tag.find("</a>") {
                    strip_html(&after_tag[..close_tag])
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if !title.is_empty() && !url.is_empty() {
            results.push(SearchResult {
                title,
                url,
                snippet,
            });
        }

        search_from = body_start + 1;
    }

    results
}

fn clean_ddg_url(url: &str) -> String {
    // DDG wraps URLs with redirect; extract the actual URL
    if let Some(rest) = url.strip_prefix("//") {
        return format!("https://{}", rest);
    }
    if let Some(rest) = url.strip_prefix("https://duckduckgo.com/l/?uddg=") {
        if let Ok(decoded) = url_decode(rest) {
            return decoded;
        }
    }
    url.to_string()
}

fn url_decode(s: &str) -> Result<String, ()> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().ok_or(())?.to_digit(16).ok_or(())?;
            let h2 = chars.next().ok_or(())?.to_digit(16).ok_or(())?;
            result.push(char::from_u32(h1 << 4 | h2).ok_or(())?);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    Ok(result)
}

fn strip_html(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Format search results for injection into conversation context.
pub fn format_results(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "（未找到搜索结果）".to_string();
    }
    let mut text = String::from("[联网搜索结果]\n\n");
    for (i, r) in results.iter().enumerate() {
        text.push_str(&format!("{}. {}\n", i + 1, r.title));
        text.push_str(&format!("   {}\n", r.url));
        if !r.snippet.is_empty() {
            text.push_str(&format!("   {}\n", r.snippet));
        }
        text.push('\n');
    }
    text.push_str("---\n请基于以上搜索结果回答用户的问题，引用来源时注明序号。");
    text
}

/// Execute DDG search (DeepSeek native web_search is handled by caller).
pub async fn execute_search(query: &str, config: &WebSearchConfig) -> anyhow::Result<String> {
    let results = search_ddg(query, config).await?;
    Ok(format_results(&results))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_escape_basic() {
        assert_eq!(url_escape("hello world"), "hello%20world");
        assert_eq!(url_escape("a-b_c.d"), "a-b_c.d");
    }

    #[test]
    fn url_escape_cjk() {
        let escaped = url_escape("中文");
        assert!(escaped.contains('%'));
        assert!(!escaped.contains('中'));
    }

    #[test]
    fn url_decode_basic() {
        assert_eq!(url_decode("hello%20world").unwrap(), "hello world");
        assert_eq!(url_decode("a+b").unwrap(), "a b");
        assert_eq!(url_decode("no-escape").unwrap(), "no-escape");
    }

    #[test]
    fn url_decode_invalid() {
        assert!(url_decode("%ZZ").is_err());
        assert!(url_decode("%2").is_err());
    }

    #[test]
    fn strip_html_basic() {
        assert_eq!(strip_html("hello"), "hello");
        assert_eq!(strip_html("<b>bold</b>"), "bold");
        assert_eq!(strip_html("a  b   c"), "a b c"); // whitespace collapse
    }

    #[test]
    fn strip_html_nested() {
        assert_eq!(strip_html("<div><span>nested</span></div>"), "nested");
    }

    #[test]
    fn clean_ddg_url_direct() {
        assert_eq!(
            clean_ddg_url("//example.com/page"),
            "https://example.com/page"
        );
    }

    #[test]
    fn clean_ddg_url_ddg_redirect() {
        let input = "https://duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com";
        assert_eq!(clean_ddg_url(input), "https://example.com");
    }

    #[test]
    fn clean_ddg_url_passthrough() {
        assert_eq!(clean_ddg_url("https://example.com"), "https://example.com");
    }

    #[test]
    fn format_results_empty() {
        let s = format_results(&[]);
        assert!(s.contains("未找到"));
    }

    #[test]
    fn format_results_with_data() {
        let results = vec![SearchResult {
            title: "Test Title".into(),
            url: "https://example.com".into(),
            snippet: "A test snippet".into(),
        }];
        let s = format_results(&results);
        assert!(s.contains("Test Title"));
        assert!(s.contains("https://example.com"));
        assert!(s.contains("A test snippet"));
        assert!(s.contains("1. "));
    }

    #[test]
    fn format_results_multiple() {
        let results: Vec<SearchResult> = (0..3)
            .map(|i| SearchResult {
                title: format!("Title {}", i),
                url: format!("https://example.com/{}", i),
                snippet: String::new(),
            })
            .collect();
        let s = format_results(&results);
        assert!(s.contains("1. Title 0"));
        assert!(s.contains("2. Title 1"));
        assert!(s.contains("3. Title 2"));
    }

    #[test]
    fn parse_ddg_html_empty() {
        let results = parse_ddg_html("<html><body></body></html>", 5);
        assert!(results.is_empty());
    }
}
