//! Fikra Code never talks to OpenAI or ChatGPT services. Requests to those
//! hosts are refused locally before any connection is made.

const BLOCKED_HOST_SUFFIXES: &[&str] = &[
    "openai.com",
    "chatgpt.com",
    "oaistatic.com",
    "oaiusercontent.com",
    "openai.org",
];

/// Returns true when `host` is, or is a subdomain of, an OpenAI/ChatGPT domain.
pub fn is_blocked_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    BLOCKED_HOST_SUFFIXES.iter().any(|suffix| {
        host == *suffix
            || host
                .strip_suffix(suffix)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

/// Returns true when the URL points at a blocked host.
pub fn is_blocked_url(url: &url::Url) -> bool {
    url.host_str().is_some_and(is_blocked_host)
}

#[cfg(test)]
mod tests {
    use super::is_blocked_host;

    #[test]
    fn blocks_openai_hosts() {
        assert!(is_blocked_host("api.openai.com"));
        assert!(is_blocked_host("chatgpt.com"));
        assert!(is_blocked_host("ab.chatgpt.com."));
        assert!(is_blocked_host("AUTH.OPENAI.COM"));
        assert!(!is_blocked_host("fikraapi.co.ke"));
        assert!(!is_blocked_host("notopenai.com"));
        assert!(!is_blocked_host("github.com"));
    }
}
