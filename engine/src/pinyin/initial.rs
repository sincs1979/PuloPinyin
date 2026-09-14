/// First-letter (and zh/ch/sh) keys of a pinyin syllable sequence.

pub fn initial_keys(syllables: &[impl AsRef<str>]) -> String {
    let mut out = String::new();
    for s in syllables {
        let s = s.as_ref();
        if s.is_empty() {
            continue;
        }
        out.push(s.chars().next().unwrap());
    }
    out
}

pub fn first_letter(syllable: &str) -> Option<char> {
    syllable.chars().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zg_from_zhong_guo() {
        assert_eq!(initial_keys(&["zhong", "guo"]), "zg");
        assert_eq!(initial_keys(&["shu", "ru", "fa"]), "srf");
        assert_eq!(initial_keys(&["ping", "guo", "shou", "ji"]), "pgsj");
    }
}
