use oa_telemetry_server::domain::{extract_domain, url_matches_domains};

#[test]
fn test_extract_domain_strips_www() {
    assert_eq!(
        extract_domain("https://www.ft.com/content/article"),
        Some("ft.com".to_string())
    );
}

#[test]
fn test_extract_domain_preserves_subdomain() {
    assert_eq!(
        extract_domain("https://tech.bbc.co.uk/news/article"),
        Some("tech.bbc.co.uk".to_string())
    );
}

#[test]
fn test_extract_domain_lowercases() {
    assert_eq!(
        extract_domain("https://WWW.Example.COM/page"),
        Some("example.com".to_string())
    );
}

#[test]
fn test_extract_domain_invalid_url() {
    assert_eq!(extract_domain("not-a-url"), None);
    assert_eq!(extract_domain(""), None);
}

#[test]
fn test_url_matches_domains_exact() {
    let domains = vec!["ft.com".to_string()];
    assert!(url_matches_domains("https://ft.com/article", &domains));
    assert!(url_matches_domains("https://www.ft.com/article", &domains));
}

#[test]
fn test_url_matches_domains_subdomain() {
    let domains = vec!["ft.com".to_string()];
    assert!(url_matches_domains("https://tech.ft.com/article", &domains));
}

#[test]
fn test_url_matches_domains_no_match() {
    let domains = vec!["ft.com".to_string()];
    assert!(!url_matches_domains("https://bbc.co.uk/news", &domains));
    assert!(!url_matches_domains("https://notft.com/page", &domains));
}

#[test]
fn test_url_matches_domains_multiple() {
    let domains = vec![
        "bbc.co.uk".to_string(),
        "ft.com".to_string(),
    ];
    assert!(url_matches_domains("https://bbc.co.uk/news", &domains));
    assert!(url_matches_domains("https://ft.com/article", &domains));
    assert!(!url_matches_domains("https://guardian.com/uk", &domains));
}
