//! Chinese punctuation mapping for Chinese mode.
//!
//! ASCII mode leaves keys untouched. Quote keys pair as ‘’ and “”.

#[derive(Debug, Clone, Default)]
pub struct QuoteState {
    double_open: bool,
    single_open: bool,
}

impl QuoteState {
    fn next_double(&mut self) -> String {
        self.double_open = !self.double_open;
        if self.double_open {
            "“".into()
        } else {
            "”".into()
        }
    }

    fn next_single(&mut self) -> String {
        self.single_open = !self.single_open;
        if self.single_open {
            "‘".into()
        } else {
            "’".into()
        }
    }
}

pub fn is_punct_key(ch: char) -> bool {
    matches!(
        ch,
        ',' | '.'
            | '?'
            | '!'
            | ':'
            | ';'
            | '\\'
            | '/'
            | '('
            | ')'
            | '['
            | ']'
            | '<'
            | '>'
            | '"'
            | '\''
            | '`'
            | '^'
            | '_'
            | '$'
            | '~'
            | '|'
            | '@'
            | '+'
            | '-'
            | '='
            | '＋'
            | '－'
            | '，'
            | '。'
            | '？'
            | '！'
            | '：'
            | '；'
            | '、'
            | '（'
            | '）'
            | '【'
            | '】'
            | '《'
            | '》'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '·'
            | '￥'
            | '～'
            | '｜'
    )
}

pub fn to_chinese(ch: char, quotes: &mut QuoteState) -> String {
    match ch {
        ',' | '，' => "，".into(),
        '.' | '。' => "。".into(),
        '?' | '？' => "？".into(),
        '!' | '！' => "！".into(),
        ':' | '：' => "：".into(),
        ';' | '；' => "；".into(),
        '\\' | '、' => "、".into(),
        '/' => "、".into(),
        '(' | '（' => "（".into(),
        ')' | '）' => "）".into(),
        '[' | '【' => "【".into(),
        ']' | '】' => "】".into(),
        '<' | '《' => "《".into(),
        '>' | '》' => "》".into(),
        '"' | '“' | '”' => quotes.next_double(),
        '\'' | '‘' | '’' => quotes.next_single(),
        '`' | '·' => "·".into(),
        '^' => "……".into(),
        '_' => "——".into(),
        '$' | '￥' => "￥".into(),
        '~' | '～' => "～".into(),
        '|' | '｜' => "｜".into(),
        other => other.to_string(),
    }
}

/// Half-width punctuation after English letters in Chinese mode.
pub fn to_ascii(ch: char) -> String {
    match ch {
        '，' => ",".into(),
        '。' => ".".into(),
        '？' => "?".into(),
        '！' => "!".into(),
        '：' => ":".into(),
        '；' => ";".into(),
        '、' => "\\".into(),
        '（' => "(".into(),
        '）' => ")".into(),
        '【' => "[".into(),
        '】' => "]".into(),
        '《' => "<".into(),
        '》' => ">".into(),
        '“' | '”' => "\"".into(),
        '‘' | '’' => "'".into(),
        '·' => "`".into(),
        '￥' => "$".into(),
        '～' => "~".into(),
        '｜' => "|".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_marks() {
        let mut q = QuoteState::default();
        assert_eq!(to_chinese(',', &mut q), "，");
        assert_eq!(to_chinese('.', &mut q), "。");
        assert_eq!(to_chinese('?', &mut q), "？");
        assert_eq!(to_chinese('$', &mut q), "￥");
        assert_eq!(to_chinese('\\', &mut q), "、");
        assert_eq!(to_ascii('.'), ".");
        assert_eq!(to_ascii('。'), ".");
        assert_eq!(to_ascii('?'), "?");
    }

    #[test]
    fn quotes_pair() {
        let mut q = QuoteState::default();
        assert_eq!(to_chinese('"', &mut q), "“");
        assert_eq!(to_chinese('"', &mut q), "”");
        assert_eq!(to_chinese('\'', &mut q), "‘");
        assert_eq!(to_chinese('\'', &mut q), "’");
    }
}
