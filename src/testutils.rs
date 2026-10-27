use std::{fs, path::Path};

use phenopackets::schema::v2::Phenopacket;
use prost::Message;
use tempfile::TempDir;

use crate::phenopacket::fs::FilePhenoStore;

/// get a temporary copy of the provided file. the copy will be deleted once the filehandle is dropped.
pub(crate) fn tmpfilecopy_from(path: &str) -> std::io::Result<tempfile::NamedTempFile> {
    let tmpfile = tempfile::NamedTempFile::new()?;
    fs::copy(Path::new(path), tmpfile.path())?;

    Ok(tmpfile)
}

/// parse the example phenopacket stored in the repo
pub(crate) fn example_phenopacket() -> Phenopacket {
    let path = "data/phenopacket.pb";
    let buf = std::fs::read(path).expect("The resource file is in the repository");
    Phenopacket::decode(&buf[..]).expect("data/phenopacket.pb should be a valid phenopacket")
}

/// create an empty store in a temporary directory.
/// the store owns the [`TempDir`], so the directory is removed once the store is dropped.
pub(crate) fn example_filephenostore() -> FilePhenoStore<TempDir> {
    let temp_dir = TempDir::new().expect("should be alowed to create temp dirs.");
    FilePhenoStore::new(temp_dir).expect("tempdir should be a valid path.")
}
