use super::{parse_tsv, BinaryDict};
use std::path::Path;

const BUILTIN_TSV: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/builtin.tsv"));

pub fn load_system_dict(path: Option<&Path>) -> crate::Result<BinaryDict> {
    if let Some(path) = path {
        if path.exists() {
            if path.extension().and_then(|s| s.to_str()) == Some("tsv") {
                let text = std::fs::read_to_string(path)?;
                return Ok(BinaryDict::from_entries(parse_tsv(&text)?));
            }
            return BinaryDict::load(path);
        }
    }
    Ok(BinaryDict::from_entries(parse_tsv(BUILTIN_TSV)?))
}

pub fn builtin_tsv() -> &'static str {
    BUILTIN_TSV
}
