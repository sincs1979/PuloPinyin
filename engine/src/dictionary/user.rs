use super::BinaryDict;
use std::path::Path;

pub fn load_user_dict(path: impl AsRef<Path>) -> crate::Result<BinaryDict> {
    let path = path.as_ref();
    if path.exists() {
        BinaryDict::load(path)
    } else {
        Ok(BinaryDict::default())
    }
}
