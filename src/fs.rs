//! A [`PhenoStore`] implementation backed by a file system.
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

use phenopackets::schema::v2::Phenopacket;
use prost::Message;

use crate::core::{PhenoStore, PhenoStoreError};

/// The reasons why creating of a [`FilePhenoStore`] can fail.
#[derive(Debug)]
pub enum FilePhenoStoreError {
    /// The provided path did not point to a directory.
    NotADirectory,
    Io(std::io::Error),
}

impl From<std::io::Error> for FilePhenoStoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for FilePhenoStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilePhenoStoreError::NotADirectory => write!(f, "Not a directory"),
            FilePhenoStoreError::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for FilePhenoStoreError {}

/// FilePhenoStore keeps all phenopackets in a user-provided file system directory.
#[derive(Debug, Clone, PartialEq)]
pub struct FilePhenoStore<P> {
    dir: P,
}

impl<P> FilePhenoStore<P>
where
    P: AsRef<Path>,
{
    /// Initiate a new File-based Phenopackets Store at given location.
    ///
    /// Will create an empty folder at the specified path, if `path` is not an existing directory.
    ///
    /// # Errors:
    ///   - if the directory does not exist and could not be created
    ///   - the path exists but it does not point to a directory
    pub fn new(path: P) -> Result<FilePhenoStore<P>, FilePhenoStoreError> {
        if path.as_ref().exists() {
            if !path.as_ref().is_dir() {
                return Err(FilePhenoStoreError::NotADirectory);
            }
        } else {
            fs::create_dir_all(&path).map_err(FilePhenoStoreError::Io)?;
        }
        Ok(FilePhenoStore { dir: path })
    }

    /// returns the path for a pb file based on the id. Does NOT check whether the file actually exists, this just implements the naming convention.
    fn get_phenopacket_path(&self, id: &Uuid) -> PathBuf {
        self.dir.as_ref().join(format!("{id}.pb"))
    }

    pub fn ls(&self) -> Result<Vec<std::fs::DirEntry>, PhenoStoreError> {
        Ok(self
            .dir
            .as_ref()
            .read_dir()?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pb"))
            .collect())
    }

    pub fn len(&self) -> Result<usize, PhenoStoreError> {
        Ok(self
            .dir
            .as_ref()
            .read_dir()?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pb"))
            .count())
    }

    pub fn is_empty(&self) -> Result<bool, PhenoStoreError> {
        Ok(self.ls().iter().len() == 0)
    }
}

impl<P> PhenoStore for FilePhenoStore<P>
where
    P: AsRef<Path>,
{
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError> {
        let id = Uuid::new_v4();
        write_phenopacket(phenopacket, &self.get_phenopacket_path(&id))?;

        Ok(id)
    }

    fn remove(&self, id: &Uuid) -> Result<bool, PhenoStoreError> {
        let path = self.get_phenopacket_path(id);
        delete_phenopacket(&path)
    }

    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError> {
        let path = self.get_phenopacket_path(id);
        match read_phenopacket(&path) {
            Ok(phenopacket) => Ok(phenopacket),
            Err(e) => Err(e),
        }
    }

    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<bool, PhenoStoreError> {
        let pp_path = self.get_phenopacket_path(id);

        match self.get(id)? {
            Some(_) => {
                replace_phenopacket(phenopacket, &pp_path)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

/// read a phenopacket from the specified path
fn read_phenopacket(path: &Path) -> Result<Option<Phenopacket>, PhenoStoreError> {
    let mut file = match File::open(path) {
        Ok(it) => it,
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => return Ok(None),
            _ => return Err(err.into()),
        },
    };
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    match Phenopacket::decode(&buf[..]) {
        Ok(pp) => Ok(Some(pp)),
        Err(_) => Err(PhenoStoreError::InvalidData),
    }
}

/// create a new protobuf file at the specified path
fn write_phenopacket(phenopacket: &Phenopacket, path: &Path) -> Result<(), PhenoStoreError> {
    // study "encoded_len" to prevent Vec reallocations.
    let mut buf = Vec::new();
    phenopacket
        .encode(&mut buf)
        .expect("we expect enough memory to store a phenopacket.");
    let mut file = File::create(path)?;
    file.write_all(&buf)?;

    Ok(())
}

/// delete a phenopacket from the specified path
fn delete_phenopacket(path: &Path) -> Result<bool, PhenoStoreError> {
    match fs::remove_file(path) {
        Ok(_) => Ok(true),
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => Ok(false),
            _ => Err(e.into()),
        },
    }
}

/// replace phenopacket at the specified path
fn replace_phenopacket(phenopacket: &Phenopacket, path: &Path) -> Result<(), PhenoStoreError> {
    let tmp_file = path.with_extension("_tmp");
    write_phenopacket(phenopacket, &tmp_file)?;
    fs::rename(&tmp_file, path).map_err(|error| {
        let _ = fs::remove_file(&tmp_file);
        PhenoStoreError::from(error)
    })?;

    Ok(())
}

//
// TESTS
//
#[cfg(test)]
mod testutils {
    use std::{fs, path::Path};
    use tempfile;

    use crate::fs::*;

    /// get a temporary copy of the provided file. the copy will be deleted once the filehandle is dropped.
    pub fn tmpfilecopy_from(path: &str) -> std::io::Result<tempfile::NamedTempFile> {
        let tmpfile = tempfile::NamedTempFile::new()?;
        fs::copy(Path::new(path), tmpfile.path())?;

        Ok(tmpfile)
    }

    /// create a new example phenopacket from data/phenopacket.pb
    pub fn example_phenopacket() -> Phenopacket {
        read_phenopacket(Path::new("data/phenopacket.pb"))
            .expect("Test phenopacket should be present in the repo")
            .expect("Test phenopacket should be present in the repo")
    }

    /// create an empty example store in a temporary directory
    /// the TempDir needs to be returned and kept alive a long as you want to use the store, otherwise the dir is removed.
    pub fn example_store_empty() -> FilePhenoStore<tempfile::TempDir> {
        let temp_dir =
            tempfile::TempDir::new().expect("It is possible to create a temporary folder");
        FilePhenoStore::new(temp_dir).expect("We just created a temporary folder and it exists")
    }
}

#[cfg(test)]
mod test_core {
    use std::path::Path;

    use super::{testutils::example_phenopacket, *};

    #[test]
    fn test_read_phenopacket() {
        let path = Path::new("data/phenopacket.pb");
        let pp = read_phenopacket(path)
            .expect("Test phenopacket should be present")
            .expect("Test phenopacket should be present");

        assert_eq!(pp.id, "comprehensive-phenopacket-id");
    }

    #[test]
    fn test_read_create_phenopacket() -> Result<(), PhenoStoreError> {
        let pp = example_phenopacket();

        // write
        let outfile = tempfile::NamedTempFile::new()?;
        write_phenopacket(&pp, outfile.path())?;
        assert!(!std::fs::metadata(outfile.path())?.len() > 0);

        // read again to check persistence
        let pp_created = read_phenopacket(outfile.path())
            .expect("The phenopacket file exists and is well formatted")
            .expect("The phenopacket file exists");
        assert_eq!(
            pp, pp_created,
            "the written phenopacket does not match the read phenopacket."
        );

        Ok(())
    }

    #[test]
    fn test_delete_phenopacket() -> Result<(), PhenoStoreError> {
        let deleteme_file = testutils::tmpfilecopy_from("data/phenopacket.pb")?;
        assert!(
            deleteme_file.path().exists(),
            "the test dummy file should exist here, but does not."
        );

        delete_phenopacket(deleteme_file.path())?;

        assert!(
            !deleteme_file.path().exists(),
            "the test dummy file should not exist anymore, but does."
        );
        Ok(())
    }

    #[test]
    fn test_update_phenopacket() -> Result<(), PhenoStoreError> {
        let pp_path = "data/phenopacket.pb";
        let file = testutils::tmpfilecopy_from(pp_path)?;
        let mut pp = example_phenopacket();

        // change some values
        pp.id = "a new id".into();

        // apply changes
        replace_phenopacket(&pp, file.path())?;

        // verify
        assert!(file.path().exists());
        let pp_orig = example_phenopacket();
        let pp_updated = read_phenopacket(file.path())
            .expect("The phenopacket was written")
            .expect("The phenopacket was written");
        assert_ne!(
            pp_orig, pp_updated,
            "no changes were applied to the stored phenopacket"
        );
        assert_eq!(pp.id, pp_updated.id);
        Ok(())
    }
}

// FilePhenoStore
#[cfg(test)]
mod test_file_pheno_store {
    use super::{testutils::example_phenopacket, *};
    use std::{assert_matches, fs};

    #[test]
    fn test_new() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::TempDir::new()?;
        let store_path = temp_dir.path().join("store");

        let store = FilePhenoStore::new(&store_path)?;

        assert!(store.dir.is_dir());
        assert_eq!(&store_path, store.dir);
        assert!(
            store.dir.read_dir()?.next().is_none(),
            "new store should be empty"
        );
        Ok(())
    }

    #[test]
    fn test_add() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::TempDir::new()?;
        let ps = FilePhenoStore::new(temp_dir.path())?;
        let pp = example_phenopacket();
        let id = ps.add(&pp)?;

        // is ps dir non empty now?
        assert!(
            ps.dir.read_dir()?.next().is_some(),
            "failed to add a pb file to PhenoStore directory."
        );
        assert!(ps.get_phenopacket_path(&id).exists());
        Ok(())
    }

    #[test]
    /// depends on add()
    fn test_remove() -> Result<(), Box<dyn std::error::Error>> {
        // setup
        let ps = testutils::example_store_empty();

        // successfully remove a stored phenopacket
        let pp = example_phenopacket();
        let id = ps
            .add(&pp)
            .expect("There is plenty space for storing a phenopacket");
        let test_exists = ps.remove(&id);
        assert_matches!(test_exists, Ok(true));
        assert!(
            !ps.get_phenopacket_path(&id).exists(),
            "pb file to delete still exists."
        );
        assert!(
            ps.dir.as_ref().read_dir()?.next().is_none(),
            "store dir not empty -> remove failed"
        );

        // try to remove non existent
        let nonexistend_id = Uuid::new_v4();
        let test_nonexists = ps.remove(&nonexistend_id);
        assert_matches!(test_nonexists, Ok(false));

        Ok(())
    }

    #[test]
    fn test_get() -> Result<(), Box<dyn std::error::Error>> {
        // setup
        let temp_dir = tempfile::TempDir::new()?;
        let ps = FilePhenoStore::new(temp_dir.path())?;

        // successfully get a stored phenopacket
        let pp = example_phenopacket();
        let id = ps.add(&pp)?;
        let test_exists = ps.get(&id);
        assert_matches!(test_exists, Ok(Some(_)));

        // valid id, non-existend in store
        let test_notfound = ps.get(&Uuid::new_v4());
        assert_matches!(test_notfound, Ok(None));

        // valid id and file name, but corrupted contents (not valid phenopacket)
        let id_corrupted_file = Uuid::new_v4();
        let mut corrupted_file = fs::File::create_new(ps.get_phenopacket_path(&id_corrupted_file))?;
        write!(
            corrupted_file,
            "ohno this file is corrupted and does not contain valid pb data :("
        )
        .expect("We can write to the file");
        std::mem::drop(corrupted_file); // flush everything to filesystem
        let test_fail = ps.get(&id_corrupted_file);
        assert_matches!(test_fail, Err(PhenoStoreError::InvalidData));

        Ok(())
    }

    #[test]
    fn test_len() -> Result<(), PhenoStoreError> {
        let ps = testutils::example_store_empty();
        let len0 = ps.len()?;

        let pp1 = example_phenopacket();
        let _ = ps.add(&pp1)?;
        let len1 = ps.len()?;

        let pp2 = example_phenopacket();
        let _ = ps.add(&pp2)?;
        let len2 = ps.len()?;

        assert_eq!(len0, 0);
        assert_eq!(len1, 1);
        assert_eq!(len2, 2);
        Ok(())
    }

    #[test]
    fn test_get_phenopacket_path() {
        let fps = FilePhenoStore::new("data").expect("The repo contains the data folder");

        let uuid: Uuid = "67e55044-10b1-426f-9247-bb680e5fe0c8"
            .parse()
            .expect("UUID is OK");
        let path = fps.get_phenopacket_path(&uuid);

        assert_eq!(
            &path.to_str().expect("Test path is a valid UTF-8 str"),
            &"data/67e55044-10b1-426f-9247-bb680e5fe0c8.pb"
        );
    }
}
