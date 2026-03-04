use std::fs::File;
use std::path::Path;

use flate2::read::GzDecoder;

use super::schema::LkRoot;
use super::LkError;

pub fn read_lk_file(path: &Path) -> Result<LkRoot, LkError> {
    let file = File::open(path)?;
    let decoder = GzDecoder::new(file);
    let root: LkRoot = serde_json::from_reader(decoder)?;
    Ok(root)
}
