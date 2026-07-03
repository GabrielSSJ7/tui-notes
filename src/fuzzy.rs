use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// Fuzzy-filter `items` by `query`, returning the matching indices ordered by
/// descending score. Empty query yields no matches (caller shows the tree).
pub fn filter(query: &str, items: &[String]) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut scored: Vec<(usize, u32)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, s)| score(&pattern, s, &mut matcher).map(|sc| (i, sc)))
        .collect();
    scored.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
    scored.into_iter().map(|(i, _)| i).collect()
}

fn score(pattern: &Pattern, haystack: &str, matcher: &mut Matcher) -> Option<u32> {
    let mut buf = Vec::new();
    let utf32 = Utf32Str::new(haystack, &mut buf);
    pattern.score(utf32, matcher)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<String> {
        ["projects/todo.md", "diary/2026.txt", "shopping.md"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn empty_query_returns_nothing() {
        assert!(filter("", &items()).is_empty());
    }

    #[test]
    fn matches_subsequence() {
        let hits = filter("todo", &items());
        assert_eq!(hits.first().copied(), Some(0));
    }

    #[test]
    fn no_match_returns_empty() {
        assert!(filter("zzzzz", &items()).is_empty());
    }
}
