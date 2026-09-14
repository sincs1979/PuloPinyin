/// Fuzzy pinyin expansion.
///
/// Rules (phase 1):
///   z  <-> zh
///   c  <-> ch
///   s  <-> sh
///   r  <-> l
///
/// Fuzzy only expands the search range. It must never be used as a ranking signal.

pub fn fuzzy_variants(syllable: &str) -> Vec<String> {
    let mut out = Vec::new();
    for zh in expand_zh_group(syllable) {
        for rl in expand_rl(&zh) {
            for ue in expand_ue_ve(&rl) {
                if !out.iter().any(|s| s == &ue) {
                    out.push(ue);
                }
            }
        }
    }
    out
}

/// Initial keys for a syllable, including fuzzy counterparts.
///
/// Examples:
///   zhong → z, zh
///   shi   → s, sh
///   si    → s, sh  (via s/sh fuzzy)
pub fn fuzzy_initials(syllable: &str) -> Vec<String> {
    let mut keys = Vec::new();
    for variant in fuzzy_variants(syllable) {
        push_unique(&mut keys, first_letter(&variant).to_string());
        if let Some(two) = two_letter_initial(&variant) {
            push_unique(&mut keys, two.to_string());
        }
    }
    keys
}

fn expand_zh_group(s: &str) -> Vec<String> {
    if let Some(rest) = s.strip_prefix("zh") {
        vec![s.to_string(), format!("z{rest}")]
    } else if let Some(rest) = s.strip_prefix("ch") {
        vec![s.to_string(), format!("c{rest}")]
    } else if let Some(rest) = s.strip_prefix("sh") {
        vec![s.to_string(), format!("s{rest}")]
    } else if let Some(rest) = s.strip_prefix('z') {
        vec![s.to_string(), format!("zh{rest}")]
    } else if let Some(rest) = s.strip_prefix('c') {
        vec![s.to_string(), format!("ch{rest}")]
    } else if let Some(rest) = s.strip_prefix('s') {
        vec![s.to_string(), format!("sh{rest}")]
    } else {
        vec![s.to_string()]
    }
}

fn expand_rl(s: &str) -> Vec<String> {
    if s == "er" {
        return vec![s.to_string()];
    }
    if let Some(rest) = s.strip_prefix('r') {
        vec![s.to_string(), format!("l{rest}")]
    } else if let Some(rest) = s.strip_prefix('l') {
        vec![s.to_string(), format!("r{rest}")]
    } else {
        vec![s.to_string()]
    }
}

/// `üe` after `l`/`n` is typed both as `lve` (v = ü) and `lue` (Sogou / 微软).
/// Do not alias `lu`↔`lv` or `nu`↔`nv` — those are different syllables (路 vs 吕).
fn expand_ue_ve(s: &str) -> Vec<String> {
    match s {
        "lue" => vec![s.to_string(), "lve".into()],
        "lve" => vec![s.to_string(), "lue".into()],
        "nue" => vec![s.to_string(), "nve".into()],
        "nve" => vec![s.to_string(), "nue".into()],
        _ => vec![s.to_string()],
    }
}

fn first_letter(s: &str) -> char {
    s.chars().next().unwrap_or('?')
}

fn two_letter_initial(s: &str) -> Option<&str> {
    if s.starts_with("zh") || s.starts_with("ch") || s.starts_with("sh") {
        Some(&s[..2])
    } else {
        None
    }
}

fn push_unique(v: &mut Vec<String>, s: String) {
    if !v.iter().any(|x| x == &s) {
        v.push(s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn si_includes_shi() {
        let v = fuzzy_variants("si");
        assert!(v.contains(&"si".into()));
        assert!(v.contains(&"shi".into()));
    }

    #[test]
    fn lan_includes_ran() {
        let v = fuzzy_variants("lan");
        assert!(v.contains(&"lan".into()));
        assert!(v.contains(&"ran".into()));
    }

    #[test]
    fn zang_includes_zhang() {
        let v = fuzzy_variants("zang");
        assert!(v.contains(&"zang".into()));
        assert!(v.contains(&"zhang".into()));
    }

    #[test]
    fn lue_and_lve_are_the_same() {
        let v = fuzzy_variants("lve");
        assert!(v.contains(&"lve".into()));
        assert!(v.contains(&"lue".into()));
        let u = fuzzy_variants("lue");
        assert!(u.contains(&"lue".into()));
        assert!(u.contains(&"lve".into()));
        assert!(!fuzzy_variants("lu").contains(&"lv".into()));
        assert!(!fuzzy_variants("lv").contains(&"lu".into()));
    }

    #[test]
    fn nue_and_nve_are_the_same() {
        assert!(fuzzy_variants("nve").contains(&"nue".into()));
        assert!(fuzzy_variants("nue").contains(&"nve".into()));
        assert!(!fuzzy_variants("nu").contains(&"nv".into()));
    }
}
