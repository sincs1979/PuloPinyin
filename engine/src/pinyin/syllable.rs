/// A valid Hanyu Pinyin syllable without tone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Syllable(pub &'static str);

/// Longest first, so greedy matching can scan in this order per starting letter.
pub fn is_syllable(s: &str) -> bool {
    set().contains(s)
}

pub fn longest_syllable_at(input: &str) -> Option<&str> {
    syllables_at(input).into_iter().next()
}

/// Valid syllable prefixes at the start of `input`, longest first.
///
/// `keneng` → `ken`, `ke`. `xian` → `xian`, `xia`, `xi`.
pub fn syllables_at(input: &str) -> Vec<&str> {
    if input.is_empty() {
        return Vec::new();
    }
    let max = input.len().min(6);
    let mut out = Vec::new();
    for len in (1..=max).rev() {
        if !input.is_char_boundary(len) {
            continue;
        }
        let cand = &input[..len];
        if is_syllable(cand) {
            out.push(cand);
        }
    }
    out
}

use std::collections::HashSet;
use std::sync::OnceLock;

static SYLLABLE_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn set() -> &'static HashSet<&'static str> {
    SYLLABLE_SET.get_or_init(|| SYLLABLES.iter().copied().collect())
}

pub fn all_syllables() -> &'static [&'static str] {
    &SYLLABLES
}

impl Syllable {
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

/// Complete toneless Hanyu Pinyin syllable table (incl. `v` for ü).
const SYLLABLES: &[&str] = &[
    "a", "ai", "an", "ang", "ao", "ba", "bai", "ban", "bang", "bao", "bei", "ben", "beng", "bi",
    "bian", "biao", "bie", "bin", "bing", "bo", "bu", "ca", "cai", "can", "cang", "cao", "ce",
    "cei", "cen", "ceng", "ci", "cong", "cou", "cu", "cuan", "cui", "cun", "cuo", "cha", "chai",
    "chan", "chang", "chao", "che", "chen", "cheng", "chi", "chong", "chou", "chu", "chua",
    "chuai", "chuan", "chuang", "chui", "chun", "chuo", "da", "dai", "dan", "dang", "dao", "de",
    "dei", "den", "deng", "di", "dia", "dian", "diao", "die", "ding", "diu", "dong", "dou", "du",
    "duan", "dui", "dun", "duo", "e", "ei", "en", "eng", "er", "fa", "fan", "fang", "fei", "fen",
    "feng", "fo", "fou", "fu", "ga", "gai", "gan", "gang", "gao", "ge", "gei", "gen", "geng",
    "gong", "gou", "gu", "gua", "guai", "guan", "guang", "gui", "gun", "guo", "ha", "hai", "han",
    "hang", "hao", "he", "hei", "hen", "heng", "hong", "hou", "hu", "hua", "huai", "huan", "huang",
    "hui", "hun", "huo", "ji", "jia", "jian", "jiang", "jiao", "jie", "jin", "jing", "jiong",
    "jiu", "ju", "juan", "jue", "jun", "ka", "kai", "kan", "kang", "kao", "ke", "kei", "ken",
    "keng", "kong", "kou", "ku", "kua", "kuai", "kuan", "kuang", "kui", "kun", "kuo", "la", "lai",
    "lan", "lang", "lao", "le", "lei", "leng", "li", "lia", "lian", "liang", "liao", "lie", "lin",
    "ling", "liu", "lo", "long", "lou", "lu", "luan", "lue", "lun", "luo", "lv", "lve", "ma",
    "mai", "man", "mang", "mao", "me", "mei", "men", "meng", "mi", "mian", "miao", "mie", "min",
    "ming", "miu", "mo", "mou", "mu", "na", "nai", "nan", "nang", "nao", "ne", "nei", "nen",
    "neng", "ni", "nian", "niang", "niao", "nie", "nin", "ning", "niu", "nong", "nou", "nu",
    "nuan", "nue", "nuo", "nv", "nve", "o", "ou", "pa", "pai", "pan", "pang", "pao", "pei", "pen",
    "peng", "pi", "pian", "piao", "pie", "pin", "ping", "po", "pou", "pu", "qi", "qia", "qian",
    "qiang", "qiao", "qie", "qin", "qing", "qiong", "qiu", "qu", "quan", "que", "qun", "ran",
    "rang", "rao", "re", "ren", "reng", "ri", "rong", "rou", "ru", "rua", "ruan", "rui", "run",
    "ruo", "sa", "sai", "san", "sang", "sao", "se", "sen", "seng", "si", "song", "sou", "su",
    "suan", "sui", "sun", "suo", "sha", "shai", "shan", "shang", "shao", "she", "shei", "shen",
    "sheng", "shi", "shou", "shu", "shua", "shuai", "shuan", "shuang", "shui", "shun", "shuo",
    "ta", "tai", "tan", "tang", "tao", "te", "tei", "teng", "ti", "tian", "tiao", "tie", "ting",
    "tong", "tou", "tu", "tuan", "tui", "tun", "tuo", "wa", "wai", "wan", "wang", "wei", "wen",
    "weng", "wo", "wu", "xi", "xia", "xian", "xiang", "xiao", "xie", "xin", "xing", "xiong", "xiu",
    "xu", "xuan", "xue", "xun", "ya", "yan", "yang", "yao", "ye", "yi", "yin", "ying", "yo",
    "yong", "you", "yu", "yuan", "yue", "yun", "za", "zai", "zan", "zang", "zao", "ze", "zei",
    "zen", "zeng", "zi", "zong", "zou", "zu", "zuan", "zui", "zun", "zuo", "zha", "zhai", "zhan",
    "zhang", "zhao", "zhe", "zhei", "zhen", "zheng", "zhi", "zhong", "zhou", "zhu", "zhua",
    "zhuai", "zhuan", "zhuang", "zhui", "zhun", "zhuo",
];

pub fn ensure_loaded() {
    let _ = set();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_syllables() {
        ensure_loaded();
        assert!(is_syllable("zhong"));
        assert!(is_syllable("guo"));
        assert!(is_syllable("shi"));
        assert!(is_syllable("si"));
        assert!(is_syllable("shu"));
        assert!(is_syllable("lv"));
        assert!(!is_syllable("zh"));
        assert!(!is_syllable("srf"));
        assert!(!is_syllable("zhongguo"));
    }

    #[test]
    fn longest_match() {
        assert_eq!(longest_syllable_at("zhongguo"), Some("zhong"));
        assert_eq!(longest_syllable_at("shurufa"), Some("shu"));
        assert_eq!(longest_syllable_at("a"), Some("a"));
        assert_eq!(longest_syllable_at("zhx"), None);
        assert_eq!(longest_syllable_at("xian"), Some("xian"));
        assert_eq!(longest_syllable_at("diao"), Some("diao"));
        assert_ne!(longest_syllable_at("xian"), Some("xi"));
        assert_ne!(longest_syllable_at("diao"), Some("di"));
        assert_eq!(syllables_at("keneng"), vec!["ken", "ke"]);
        assert_eq!(syllables_at("xian")[0], "xian");
    }

    #[test]
    fn table_is_unique() {
        let mut v = SYLLABLES.to_vec();
        v.sort_unstable();
        v.dedup();
        assert_eq!(v.len(), SYLLABLES.len());
    }
}
