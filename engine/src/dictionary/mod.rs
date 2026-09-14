pub mod binary;
pub mod system;
pub mod user;

pub use binary::{parse_tsv, BinaryDict, DictEntry, MAGIC, VERSION};
pub use system::load_system_dict;
pub use user::load_user_dict;
