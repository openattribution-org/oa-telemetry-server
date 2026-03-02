use url::Url;
use uuid::Uuid;

use crate::state::OaState;

/// Extract the registrable domain from a URL.
///
/// Strips `www.` prefix and returns the hostname. For subdomain matching,
/// we check both the exact domain and parent domains against the index.
///
/// # Examples
///
/// - `https://www.bbc.co.uk/news/article` -> `bbc.co.uk`
/// - `https://tech.ft.com/article` -> `ft.com` (if `ft.com` is registered, not `tech.ft.com`)
pub fn extract_domain(url_str: &str) -> Option<String> {
    let url = Url::parse(url_str).ok()?;
    let host = url.host_str()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    Some(host.to_lowercase())
}

/// Resolve a content URL to its publisher using the domain index.
///
/// Walks up the domain hierarchy to find a match:
/// 1. Try exact domain (e.g., `tech.ft.com`)
/// 2. Try parent domain (e.g., `ft.com`)
pub fn resolve_publisher(state: &OaState, url_str: &str) -> Option<Uuid> {
    let domain = extract_domain(url_str)?;

    // Exact match first
    if let Some(entry) = state.domain_index.get(&domain) {
        return Some(*entry.value());
    }

    // Walk up domain hierarchy
    let parts: Vec<&str> = domain.split('.').collect();
    if parts.len() > 2 {
        // Try parent: e.g., tech.ft.com -> ft.com
        let parent = parts[1..].join(".");
        if let Some(entry) = state.domain_index.get(&parent) {
            return Some(*entry.value());
        }
    }

    None
}

/// Check whether any of the publisher's domains match the given URL.
pub fn url_matches_domains(url_str: &str, domains: &[String]) -> bool {
    let Some(domain) = extract_domain(url_str) else {
        return false;
    };

    for registered in domains {
        let registered = registered
            .strip_prefix("www.")
            .unwrap_or(registered)
            .to_lowercase();
        if domain == registered || domain.ends_with(&format!(".{registered}")) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain_basic() {
        assert_eq!(
            extract_domain("https://www.bbc.co.uk/news"),
            Some("bbc.co.uk".to_string())
        );
    }

    #[test]
    fn test_extract_domain_subdomain() {
        assert_eq!(
            extract_domain("https://tech.ft.com/article"),
            Some("tech.ft.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_no_www() {
        assert_eq!(
            extract_domain("https://example.com/page"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_invalid() {
        assert_eq!(extract_domain("not-a-url"), None);
    }

    #[test]
    fn test_url_matches_domains() {
        let domains = vec!["ft.com".to_string(), "bbc.co.uk".to_string()];
        assert!(url_matches_domains("https://www.ft.com/article", &domains));
        assert!(url_matches_domains("https://tech.ft.com/page", &domains));
        assert!(url_matches_domains("https://www.bbc.co.uk/news", &domains));
        assert!(!url_matches_domains("https://guardian.com/news", &domains));
    }
}
