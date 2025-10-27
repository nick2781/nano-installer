// Result 类型别名

use super::error::Error;

pub type Result<T> = std::result::Result<T, Error>;
