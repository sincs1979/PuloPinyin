use super::syllable::{is_syllable, longest_syllable_at, syllables_at};

/// One cut of a pinyin string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    /// A complete valid syllable, e.g. `zhong`.
    Full(String),
    /// A single-letter (or zh/ch/sh) initial used for phrase input.
    Initial(String),
}

impl Segment {
    pub fn as_str(&self) -> &str {
        match self {
            Segment::Full(s) | Segment::Initial(s) => s,
        }
    }
}

pub fn is_separator(c: char) -> bool {
    matches!(c, '\'' | '\u{2018}' | '\u{2019}')
}

pub fn skip_separators(s: &str) -> &str {
    s.trim_start_matches(is_separator)
}

/// Letters until the next explicit syllable separator.
pub fn chunk_letters(s: &str) -> &str {
    let s = skip_separators(s);
    match s.find(is_separator) {
        Some(i) => &s[..i],
        None => s,
    }
}

/// Greedy longest-syllable parse. Leftover letters become initials.
///
/// `'` / `’` are hard syllable boundaries: `xian` stays one syllable,
/// `xi'an` is `xi` + `an`. Used for preedit display. Dictionary lookup
/// queries every valid full-syllable cut (`keneng` → ken+eng and ke+neng).
pub fn parse_greedy(input: &str) -> Vec<Segment> {
    let input = normalize_query(input);
    let mut rest = input.as_str();
    let mut out = Vec::new();
    while !rest.is_empty() {
        if rest.starts_with('\'') {
            rest = &rest[1..];
            continue;
        }
        if let Some(syl) = longest_syllable_at(chunk_letters(rest)) {
            let len = syl.len();
            out.push(Segment::Full(syl.to_string()));
            rest = &rest[len..];
            continue;
        }
        if rest.starts_with("zh") || rest.starts_with("ch") || rest.starts_with("sh") {
            out.push(Segment::Initial(rest[..2].to_string()));
            rest = &rest[2..];
            continue;
        }
        let ch = rest.chars().next().unwrap();
        out.push(Segment::Initial(ch.to_string()));
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// Strip separators and map `ü`/`u:` to `v`.
pub fn normalize(input: &str) -> String {
    normalize_query(input).replace('\'', "")
}

/// Like [`normalize`], but keeps `'` as a syllable boundary (curly quotes folded).
pub fn normalize_query(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'ü' | 'Ü' => out.push('v'),
            'u' | 'U' if matches!(chars.peek(), Some(':')) => {
                chars.next();
                out.push('v');
            }
            'A'..='Z' => out.push(c.to_ascii_lowercase()),
            'a'..='z' => out.push(c),
            c if is_separator(c) => {
                if !out.ends_with('\'') {
                    out.push('\'');
                }
            }
            ' ' => {
                if !out.ends_with('\'') {
                    out.push('\'');
                }
            }
            _ => {}
        }
    }
    out
}

pub fn is_valid_full_pinyin(input: &str) -> bool {
    let segs = parse_greedy(input);
    !segs.is_empty() && segs.iter().all(|s| matches!(s, Segment::Full(_)))
}

pub fn join_preedit(segments: &[Segment]) -> String {
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            match seg {
                Segment::Full(_) => out.push('\''),
                Segment::Initial(_) => {}
            }
        }
        out.push_str(seg.as_str());
    }
    out
}

/// Preedit text: keep a user-typed separator; otherwise insert greedy quotes.
pub fn preedit_marked(composing: &str) -> String {
    if composing.is_empty() {
        return String::new();
    }
    if composing.chars().any(is_separator) {
        normalize_query(composing)
    } else {
        join_preedit(&parse_greedy(composing))
    }
}

/// Taking `taken` from the start of `input` must not split an *atomic* syllable.
///
/// A letter-chunk (until `'`) that is itself a valid syllable is atomic:
/// `xian` / `diao` must be taken whole. Apostrophe always forces a boundary.
///
/// In a longer run (`xianshi`), a longer valid syllable is also atomic: `xi`
/// is rejected because the leftover of `xian` is itself a syllable (`an`).
///
/// If the leftover of the longest syllable is *not* a syllable (`keneng`:
/// `ke` leaves `n`), the shorter cut is also a valid full-syllable
/// partition (`ke`+`neng`) alongside greedy `ken`+`eng`.
pub fn respects_greedy_syllable(input: &str, taken: &str) -> bool {
    let input = skip_separators(input);
    if taken.is_empty() || !input.starts_with(taken) {
        return false;
    }
    let after = &input[taken.len()..];
    if after.is_empty() || after.starts_with(is_separator) {
        return true;
    }
    let chunk = chunk_letters(input);
    if is_syllable(chunk) {
        return taken.len() == chunk.len();
    }
    !splits_longer_syllable(chunk, taken)
}

/// `taken` is a proper prefix of the longest syllable, and the leftover of
/// that syllable is itself a syllable (`xian` → `xi` + `an`).
pub fn splits_longer_syllable(chunk: &str, taken: &str) -> bool {
    let Some(longest) = longest_syllable_at(chunk) else {
        return false;
    };
    if taken.len() >= longest.len() || !longest.starts_with(taken) {
        return false;
    }
    is_syllable(&longest[taken.len()..])
}

/// `taken` is shorter than the greedy-longest syllable at the start of `input`.
///
/// After this cut, the next syllable must be a *full* syllable (absorb the
/// leftover letter: `ke`+`neng`) — not an initial that skips medials
/// (`da` + `n` for 牛 in `danshi`).
pub fn shortens_longest_syllable(input: &str, taken: &str) -> bool {
    let input = skip_separators(input);
    if taken.is_empty() || !input.starts_with(taken) {
        return false;
    }
    let chunk = chunk_letters(input);
    let Some(longest) = longest_syllable_at(chunk) else {
        return false;
    };
    taken.len() < longest.len() && longest.starts_with(taken)
}

/// Complete-syllable cuts of `input`. Greedy longest is first.
///
/// Atomic chunks stay whole (`xian` → only `xian`). Otherwise every valid
/// syllable prefix is tried (`keneng` → `ken`+`eng` and `ke`+`neng`).
/// Left-to-right DP, capped so `an`/`a` style strings cannot explode.
pub fn parse_full_cuts(input: &str) -> Vec<Vec<String>> {
    let input = normalize_query(input);
    if input.is_empty() || input.chars().all(is_separator) {
        return Vec::new();
    }
    let mut memo: Vec<Option<Vec<Vec<String>>>> = vec![None; input.len() + 1];
    let mut out = dp_full_cuts(&input, 0, &mut memo);
    out.retain(|p| !p.is_empty());
    out
}

const MAX_FULL_CUTS: usize = 16;

fn dp_full_cuts(
    input: &str,
    pos: usize,
    memo: &mut [Option<Vec<Vec<String>>>],
) -> Vec<Vec<String>> {
    if pos > input.len() {
        return Vec::new();
    }
    if let Some(cached) = &memo[pos] {
        return cached.clone();
    }
    let rest = skip_separators(&input[pos..]);
    let skipped = input[pos..].len() - rest.len();
    let start = pos + skipped;
    let result = if rest.is_empty() {
        vec![Vec::new()]
    } else {
        let chunk = chunk_letters(rest);
        let cuts: Vec<&str> = if is_syllable(chunk) {
            vec![chunk]
        } else {
            syllables_at(chunk)
                .into_iter()
                .filter(|syl| respects_greedy_syllable(rest, syl))
                .collect()
        };
        let mut result = Vec::new();
        for syl in cuts {
            if result.len() >= MAX_FULL_CUTS {
                break;
            }
            let next = start + syl.len();
            if next > input.len() {
                continue;
            }
            for tail in dp_full_cuts(input, next, memo) {
                if result.len() >= MAX_FULL_CUTS {
                    break;
                }
                let mut path = Vec::with_capacity(1 + tail.len());
                path.push(syl.to_string());
                path.extend(tail);
                result.push(path);
            }
        }
        result
    };
    memo[pos] = Some(result.clone());
    result
}

/// Raw input plus apostrophe-joined full-syllable partitions.
///
/// `keneng` → `keneng`, `ken'eng`, `ke'neng`. A forced `'` stays one query.
/// Used so lookup does not pick a single cut first.
pub fn full_cut_queries(input: &str) -> Vec<String> {
    let input = normalize_query(input);
    if input.is_empty() || input.chars().all(is_separator) {
        return Vec::new();
    }
    let mut out = vec![input.clone()];
    for cut in parse_full_cuts(&input) {
        let q = cut.join("'");
        if !out.iter().any(|s| s == &q) {
            out.push(q);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greedy_zhongguo() {
        let segs = parse_greedy("zhongguo");
        assert_eq!(
            segs,
            vec![Segment::Full("zhong".into()), Segment::Full("guo".into())]
        );
    }

    #[test]
    fn greedy_shurufa() {
        let segs = parse_greedy("shurufa");
        assert_eq!(
            segs.iter()
                .map(|s| s.as_str().to_string())
                .collect::<Vec<_>>(),
            vec!["shu", "ru", "fa"]
        );
    }

    #[test]
    fn greedy_zg() {
        let segs = parse_greedy("zg");
        assert!(segs.iter().all(|s| matches!(s, Segment::Initial(_))));
    }

    #[test]
    fn greedy_xian_is_one_syllable() {
        assert_eq!(parse_greedy("xian"), vec![Segment::Full("xian".into())]);
        assert_ne!(
            parse_greedy("xian"),
            parse_greedy("xi an"),
            "a space is a separator; xian must not become xi+an"
        );
    }

    #[test]
    fn greedy_xi_an_needs_separator() {
        assert_eq!(
            parse_greedy("xi'an"),
            vec![Segment::Full("xi".into()), Segment::Full("an".into())]
        );
        assert_eq!(
            parse_greedy("xi’an"),
            vec![Segment::Full("xi".into()), Segment::Full("an".into())]
        );
    }

    #[test]
    fn greedy_diao_is_one_syllable() {
        assert_eq!(parse_greedy("diao"), vec![Segment::Full("diao".into())]);
    }

    #[test]
    fn greedy_di_ao_needs_separator() {
        assert_eq!(
            parse_greedy("di'ao"),
            vec![Segment::Full("di".into()), Segment::Full("ao".into())]
        );
    }

    #[test]
    fn greedy_xian_an_keeps_xian() {
        assert_eq!(
            parse_greedy("xian'an"),
            vec![Segment::Full("xian".into()), Segment::Full("an".into())]
        );
    }

    #[test]
    fn normalize_strips_quote() {
        assert_eq!(normalize("zhong'guo"), "zhongguo");
        assert_eq!(normalize("ZHONGGUO"), "zhongguo");
        assert!(crate::pinyin::is_syllable("zhong"));
    }

    #[test]
    fn normalize_query_keeps_separator() {
        assert_eq!(normalize_query("xi'an"), "xi'an");
        assert_eq!(normalize_query("xi’an"), "xi'an");
        assert_eq!(normalize_query("xi an"), "xi'an");
    }

    #[test]
    fn respects_greedy_xian_and_diao() {
        assert!(!respects_greedy_syllable("xian", "xi"));
        assert!(respects_greedy_syllable("xi'an", "xi"));
        assert!(!respects_greedy_syllable("diao", "di"));
        assert!(respects_greedy_syllable("di'ao", "di"));
        assert!(respects_greedy_syllable("zhongguo", "zhong"));
    }

    #[test]
    fn respects_ke_neng_without_apostrophe() {
        assert!(respects_greedy_syllable("keneng", "ke"));
        assert!(respects_greedy_syllable("keneng", "ken"));
        assert!(respects_greedy_syllable("ke'neng", "ke"));
        assert!(is_syllable("xian"));
        assert!(!is_syllable("keneng"));
        assert!(shortens_longest_syllable("keneng", "ke"));
        assert!(!shortens_longest_syllable("keneng", "ken"));
    }

    #[test]
    fn respects_xianshi_does_not_split_xian() {
        assert!(!respects_greedy_syllable("xianshi", "xi"));
        assert!(respects_greedy_syllable("xianshi", "xian"));
        assert!(respects_greedy_syllable("xi'anshi", "xi"));
        assert!(splits_longer_syllable("xianshi", "xi"));
        assert!(!splits_longer_syllable("xianshi", "xian"));
        assert!(!splits_longer_syllable("keneng", "ke"));
        assert!(shortens_longest_syllable("danshi", "da"));
        assert!(!shortens_longest_syllable("danshi", "dan"));
        assert!(respects_greedy_syllable("danshi", "da"));
        assert!(respects_greedy_syllable("danshi", "dan"));
    }

    #[test]
    fn full_cuts_keneng_vs_xian() {
        assert_eq!(
            parse_full_cuts("keneng"),
            vec![
                vec!["ken".to_string(), "eng".to_string()],
                vec!["ke".to_string(), "neng".to_string()],
            ]
        );
        assert_eq!(parse_full_cuts("xian"), vec![vec!["xian".to_string()]]);
        assert_eq!(parse_full_cuts("diao"), vec![vec!["diao".to_string()]]);
        assert_eq!(
            parse_full_cuts("xi'an"),
            vec![vec!["xi".to_string(), "an".to_string()]]
        );
        assert_eq!(
            parse_full_cuts("ke'neng"),
            vec![vec!["ke".to_string(), "neng".to_string()]]
        );
        assert_eq!(
            parse_full_cuts("xianshi"),
            vec![vec!["xian".to_string(), "shi".to_string()]]
        );
        assert_eq!(
            parse_full_cuts("danshi"),
            vec![vec!["dan".to_string(), "shi".to_string()]]
        );
        assert_eq!(
            parse_full_cuts("xi'anshi"),
            vec![vec![
                "xi".to_string(),
                "an".to_string(),
                "shi".to_string()
            ]]
        );
        assert_eq!(
            full_cut_queries("keneng"),
            vec![
                "keneng".to_string(),
                "ken'eng".to_string(),
                "ke'neng".to_string(),
            ]
        );
        assert_eq!(full_cut_queries("xian"), vec!["xian".to_string()]);
        assert_eq!(full_cut_queries("xi'an"), vec!["xi'an".to_string()]);
        assert_eq!(full_cut_queries("ke'neng"), vec!["ke'neng".to_string()]);
    }

    #[test]
    fn full_cuts_are_capped() {
        // `an`/`a` branches would otherwise be exponential.
        let cuts = parse_full_cuts("anananananananan");
        assert!(!cuts.is_empty());
        assert!(
            cuts.len() <= MAX_FULL_CUTS,
            "expected at most {MAX_FULL_CUTS} cuts, got {}",
            cuts.len()
        );
        assert_eq!(cuts[0].as_slice(), ["an", "an", "an", "an", "an", "an", "an", "an"]);
    }
}
