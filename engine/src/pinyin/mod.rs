pub mod fuzzy;
pub mod initial;
pub mod parser;
pub mod syllable;

pub use fuzzy::{fuzzy_initials, fuzzy_variants};
pub use initial::initial_keys;
pub use parser::{full_cut_queries, parse_full_cuts, parse_greedy, Segment};
pub use syllable::{is_syllable, Syllable};

use parser::{
    is_separator, normalize_query, respects_greedy_syllable, shortens_longest_syllable,
    skip_separators,
};

/// Does `input` fully match `syllables` (full pinyin, initials, mixed, and fuzzy)?
///
/// Initial matching is enabled only when the word has two or more syllables.
/// Fuzzy forms expand the search range only — they do not affect ranking.
/// An explicit `'` is a syllable boundary (`xi'an` → 西安, `ke'neng` → 可能).
/// A longer valid syllable is not split (`xian` → 先, `xianshi` → 显示,
/// not 西安 / 西安市). Initials must not steal leftover letters of a
/// shortened syllable (`danshi` ↛ 大牛市). Alternative full-syllable cuts
/// are all valid (`keneng` → ke+neng and ken+eng); ranking is by score.
pub fn matches_word(input: &str, syllables: &[impl AsRef<str>]) -> bool {
    let input = normalize_query(input);
    match_consumed_norm(&input, syllables) == Some(input.len())
}

/// If `syllables` fully match a prefix of `input`, bytes of `input` consumed.
///
/// Leftover input is allowed — that is the prefix-candidate path (`wode` in `wodemaya`).
/// The returned length is in [`normalize_query`] space (letters + `'`).
pub fn match_consumed(input: &str, syllables: &[impl AsRef<str>]) -> Option<usize> {
    let input = normalize_query(input);
    match_consumed_norm(&input, syllables)
}

fn match_consumed_norm(input: &str, syllables: &[impl AsRef<str>]) -> Option<usize> {
    if input.is_empty() || syllables.is_empty() {
        return None;
    }
    let allow_initial = syllables.len() >= 2;
    consume_from(input, syllables, allow_initial, input.len(), true)
}

/// True when `input` is an unfinished prefix of the word (e.g. `wod` → 我的).
///
/// Single-letter input still does not expand a one-syllable word (`s` ↛ 是).
pub fn matches_prefix(input: &str, syllables: &[impl AsRef<str>]) -> bool {
    let input = normalize_query(input);
    if input.is_empty() || syllables.is_empty() {
        return false;
    }
    let allow_initial = syllables.len() >= 2;
    prefix_from(&input, syllables, allow_initial, true, true)
}

/// Greedy-longest full syllables of the consumed prefix match `syllables`
/// (fuzzy-equivalent). Matching already rejects `xian`→`xi`+`an` and
/// initial-skip (`danshi` ↛ 大牛市); this is kept for tests and diagnostics.
pub fn greedy_syllable_cover(
    input: &str,
    syllables: &[impl AsRef<str>],
    consumed: usize,
) -> bool {
    let input = normalize_query(input);
    if consumed == 0 || consumed > input.len() || !input.is_char_boundary(consumed) {
        return false;
    }
    let segs = parse_greedy(&input[..consumed]);
    if segs.is_empty() || segs.iter().any(|s| matches!(s, Segment::Initial(_))) {
        return false;
    }
    if segs.len() != syllables.len() {
        return false;
    }
    segs.iter().zip(syllables.iter()).all(|(seg, syl)| {
        fuzzy_variants(syl.as_ref())
            .iter()
            .any(|v| v.as_str() == seg.as_str())
    })
}

fn consume_from(
    input: &str,
    syllables: &[impl AsRef<str>],
    allow_initial: bool,
    orig_len: usize,
    initials_ok: bool,
) -> Option<usize> {
    if syllables.is_empty() {
        let rest = skip_separators(input);
        return Some(orig_len - rest.len());
    }
    let input = skip_separators(input);
    if input.is_empty() {
        return None;
    }

    let head = syllables[0].as_ref();
    let rest = &syllables[1..];

    for variant in fuzzy_variants(head) {
        if let Some(tail) = input.strip_prefix(variant.as_str()) {
            if respects_greedy_syllable(input, variant.as_str()) {
                let next_initials = !shortens_longest_syllable(input, variant.as_str());
                if let Some(n) = consume_from(tail, rest, allow_initial, orig_len, next_initials)
                {
                    return Some(n);
                }
            }
        }
    }

    if allow_initial && initials_ok {
        for key in fuzzy_initials(head) {
            if key.is_empty() {
                continue;
            }
            if let Some(tail) = input.strip_prefix(key.as_str()) {
                if respects_greedy_syllable(input, key.as_str()) {
                    if let Some(n) = consume_from(tail, rest, allow_initial, orig_len, true) {
                        return Some(n);
                    }
                }
            }
        }
    }

    None
}

fn prefix_from(
    input: &str,
    syllables: &[impl AsRef<str>],
    allow_initial: bool,
    first_syllable: bool,
    initials_ok: bool,
) -> bool {
    let input = skip_separators(input);
    if input.is_empty() {
        // Reached only after a full syllable was consumed (`wo` → 我们).
        return true;
    }
    if syllables.is_empty() {
        return false;
    }

    let head = syllables[0].as_ref();
    let rest = &syllables[1..];

    for variant in fuzzy_variants(head) {
        if let Some(tail) = input.strip_prefix(variant.as_str()) {
            if respects_greedy_syllable(input, variant.as_str()) {
                let next_initials = !shortens_longest_syllable(input, variant.as_str());
                if prefix_from(tail, rest, allow_initial, false, next_initials) {
                    return true;
                }
            }
        }
        // Unfinished syllable: `zhon` → 中国, `wod` → 我的.
        // A lone first letter must not expand (`z` ↛ every zh/z word).
        // A typed separator means this chunk is finished — do not keep
        // stretching into `xian` after `xi'`.
        if !input.chars().any(is_separator)
            && variant.starts_with(input)
            && input.len() < variant.len()
        {
            if first_syllable && input.len() == 1 {
                continue;
            }
            return true;
        }
    }

    if allow_initial && initials_ok {
        for key in fuzzy_initials(head) {
            if key.is_empty() {
                continue;
            }
            if let Some(tail) = input.strip_prefix(key.as_str()) {
                if !respects_greedy_syllable(input, key.as_str()) {
                    continue;
                }
                // Eating one initial and stopping is not a prefix of a longer word
                // (`z` ↛ 中国). Leftover letters can still prefix the next syllable (`zg`).
                if skip_separators(tail).is_empty() && !rest.is_empty() {
                    continue;
                }
                if prefix_from(tail, rest, allow_initial, false, true) {
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn syls(s: &str) -> Vec<String> {
        s.split_whitespace().map(|x| x.to_string()).collect()
    }

    #[test]
    fn full_pinyin_zhongguo() {
        assert!(matches_word("zhongguo", &syls("zhong guo")));
    }

    #[test]
    fn initials_zg() {
        assert!(matches_word("zg", &syls("zhong guo")));
        assert!(matches_word("zg", &syls("zhe ge")));
        assert!(matches_word("zg", &syls("zi ge")));
        assert!(matches_word("zg", &syls("zui gao")));
    }

    #[test]
    fn mixed_shurufa() {
        let w = syls("shu ru fa");
        assert!(matches_word("srf", &w));
        assert!(matches_word("shurf", &w));
        assert!(matches_word("srfa", &w));
        assert!(matches_word("shurfa", &w));
        assert!(matches_word("shurufa", &w));
    }

    #[test]
    fn single_letter_does_not_expand_chars() {
        assert!(!matches_word("s", &syls("shi")));
        assert!(!matches_word("s", &syls("si")));
    }

    #[test]
    fn fuzzy_si_shi() {
        assert!(matches_word("si", &syls("shi")));
        assert!(matches_word("si", &syls("si")));
        assert!(matches_word("shi", &syls("si")));
    }

    #[test]
    fn fuzzy_lan_ran() {
        assert!(matches_word("lan", &syls("ran")));
        assert!(matches_word("ran", &syls("lan")));
    }

    #[test]
    fn fuzzy_zang_zhang() {
        assert!(matches_word("zang", &syls("zhang")));
        assert!(matches_word("zhang", &syls("zang")));
    }

    #[test]
    fn consumed_wode_in_wodemaya() {
        assert_eq!(match_consumed("wodemaya", &syls("wo de")), Some(4));
        assert_eq!(match_consumed("wodemaya", &syls("wo")), Some(2));
        assert_eq!(match_consumed("wodemaya", &syls("ma ya")), None);
    }

    #[test]
    fn prefix_wod_is_wode() {
        assert!(matches_prefix("wod", &syls("wo de")));
        assert!(!matches_prefix("s", &syls("shi")));
    }

    #[test]
    fn first_letter_does_not_prefix_every_word() {
        assert!(!matches_prefix("z", &syls("zhong guo")));
        assert!(!matches_prefix("w", &syls("wo de")));
        assert!(!matches_prefix("n", &syls("ni hao")));
        assert!(!matches_prefix("z", &syls("zhong")));
        assert!(matches_prefix("zh", &syls("zhong guo")));
        assert!(matches_prefix("zhon", &syls("zhong guo")));
        assert!(matches_prefix("wo", &syls("wo men")));
        assert!(matches_word("zg", &syls("zhong guo")));
    }

    #[test]
    fn initials_hit_learned_phrase() {
        assert!(matches_word("wdmy", &syls("wo de ma ya")));
        assert!(matches_word("wodemaya", &syls("wo de ma ya")));
        assert!(matches_word("wdemaya", &syls("wo de ma ya")));
        assert!(matches_word("wodmy", &syls("wo de ma ya")));
    }

    #[test]
    fn xian_does_not_match_xi_an() {
        assert!(matches_word("xian", &syls("xian")));
        assert!(!matches_word("xian", &syls("xi an")));
        assert!(!matches_prefix("xian", &syls("xi an")));
        assert!(matches_word("xi'an", &syls("xi an")));
        assert!(matches_word("xi an", &syls("xi an")));
        assert_eq!(match_consumed("xi'anwo", &syls("xi an")), Some(5));
    }

    #[test]
    fn diao_does_not_match_di_ao() {
        assert!(matches_word("diao", &syls("diao")));
        assert!(!matches_word("diao", &syls("di ao")));
        assert!(!matches_prefix("diao", &syls("di ao")));
        assert!(matches_word("di'ao", &syls("di ao")));
    }

    #[test]
    fn keneng_matches_ke_neng_without_apostrophe() {
        assert!(matches_word("keneng", &syls("ke neng")));
        assert!(matches_word("ke'neng", &syls("ke neng")));
        assert!(matches_word("keneng", &syls("ken eng")));
        assert_eq!(match_consumed("keneng", &syls("ke neng")), Some(6));
        assert!(!matches_word("xian", &syls("xi an")));
    }

    #[test]
    fn xianshi_matches_xian_shi_not_xi_an_shi() {
        assert!(matches_word("xianshi", &syls("xian shi")));
        assert!(!matches_word("xianshi", &syls("xi an shi")));
        assert!(!matches_prefix("xianshi", &syls("xi an shi")));
        assert!(matches_word("xi'anshi", &syls("xi an shi")));
        assert!(matches_word("xi'an shi", &syls("xi an shi")));
        assert!(greedy_syllable_cover("xianshi", &syls("xian shi"), 7));
        assert!(!greedy_syllable_cover("xianshi", &syls("xi an shi"), 7));
    }

    #[test]
    fn danshi_matches_dan_shi_not_da_niu_shi() {
        assert!(matches_word("danshi", &syls("dan shi")));
        assert!(!matches_word("danshi", &syls("da niu shi")));
        assert!(!matches_prefix("danshi", &syls("da niu shi")));
        assert!(matches_word("daniushi", &syls("da niu shi")));
        assert!(matches_word("dnshi", &syls("da niu shi")));
        assert!(greedy_syllable_cover("danshi", &syls("dan shi"), 6));
        assert!(!greedy_syllable_cover("keneng", &syls("ke neng"), 6));
    }
}
