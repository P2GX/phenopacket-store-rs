use std::{
    fs::{self, File},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

use phenopackets::schema::v2::Phenopacket;
use prost::Message;

/// The error returned when a non-normal execution happens, e.g. network connectivity issue,
/// but not when a phenopacket is searched and not found.
#[derive(Debug)]
#[non_exhaustive]
pub enum PhenoStoreError {
    NotFound,
    AlreadyExists,
    InvalidInput,
    InvalidData,

    Storage(StorageError),

    Other,
}

#[derive(Debug)]
pub enum StorageError {
    PermissionDenied,
    ReadOnly,
    CapacityExceeded,
    Unavailable,
    Timeout,
    Initialization,
    Corrupted,
    Other,
}

impl From<IoError> for PhenoStoreError {
    fn from(error: IoError) -> Self {
        match error.kind() {
            IoErrorKind::AlreadyExists => Self::AlreadyExists,
            IoErrorKind::NotFound => Self::NotFound,
            IoErrorKind::InvalidData => Self::InvalidData,
            IoErrorKind::InvalidInput => Self::InvalidInput,

            IoErrorKind::PermissionDenied => Self::Storage(StorageError::PermissionDenied),
            IoErrorKind::ReadOnlyFilesystem => Self::Storage(StorageError::ReadOnly),
            //TODO: add other storage errors
            _ => Self::Other,
        }
    }
}

pub trait PhenoStore {
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError>;
    fn remove(&self, id: &Uuid) -> Result<(), PhenoStoreError>;
    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<(), PhenoStoreError>;
    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError>;
}

pub struct FilePhenoStore<P> {
    dir: P,
}

impl<P> FilePhenoStore<P>
where
    P: AsRef<Path>,
{
    /// Initiate a new File-based Phenopackets Store at given location.
    ///
    /// Will create an empty folder at the specified path to store phenopackets.
    ///
    /// Errors:
    ///   - PhenoStoreError::Storage(StorageError::Initialization) : if the directory could not be created or is not empty
    pub fn new(path: P) -> Result<FilePhenoStore<P>, PhenoStoreError> {
        fs::create_dir_all(&path)?;
        if fs::read_dir(&path)?.next().is_some() {
            return Err(PhenoStoreError::Storage(StorageError::Initialization));
        }

        Ok(FilePhenoStore { dir: path })
    }

    /// Create a file-based Phenopackets Store from an existing directory of phenopacket files
    pub fn open(dir: P) -> Result<FilePhenoStore<P>, PhenoStoreError> {
        match fs::read_dir(&dir) {
            Ok(_) => Ok(FilePhenoStore { dir }),
            Err(e) => Err(PhenoStoreError::from(e)),
        }
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
    /// add a Phenopacket to the store
    ///
    /// writes a file and uses the id as filename. This is a store id and independent of any id contained in the phenopacket
    fn add(&self, phenopacket: &Phenopacket) -> Result<Uuid, PhenoStoreError> {
        let id = Uuid::new_v4();
        write_phenopacket(phenopacket, &self.get_phenopacket_path(&id))?;

        Ok(id)
    }

    fn remove(&self, id: &Uuid) -> Result<(), PhenoStoreError> {
        let path = self.get_phenopacket_path(id);
        delete_phenopacket(&path)
    }

    /// retrieve a phenopacket from its id, if it exists
    /// returns Ok(None) If given valid id, but not found.
    fn get(&self, id: &Uuid) -> Result<Option<Phenopacket>, PhenoStoreError> {
        let path = self.get_phenopacket_path(id);
        match read_phenopacket(&path) {
            Ok(phenopacket) => Ok(Some(phenopacket)),
            Err(PhenoStoreError::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// If a phenopacket is updated, the previous state is gone. It is not the responsibility of this code to keep track of everything.
    fn update(&self, id: &Uuid, phenopacket: &Phenopacket) -> Result<(), PhenoStoreError> {
        let pp_path = self.get_phenopacket_path(id);

        match self.get(id)? {
            Some(_) => replace_phenopacket(phenopacket, &pp_path),
            None => Err(PhenoStoreError::NotFound),
        }
    }
}

/// read a phenopacket from the specified path
fn read_phenopacket(path: &Path) -> Result<Phenopacket, PhenoStoreError> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Phenopacket::decode(&buf[..]).map_err(|_| PhenoStoreError::Storage(StorageError::Corrupted))
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
fn delete_phenopacket(path: &Path) -> Result<(), PhenoStoreError> {
    fs::remove_file(path)?;
    Ok(())
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

    use crate::core::*;

    /// get a temporary copy of the provided file. the copy will be deleted once the filehandle is dropped.
    pub fn tmpfilecopy_from(path: &str) -> std::io::Result<tempfile::NamedTempFile> {
        let tmpfile = tempfile::NamedTempFile::new()?;
        fs::copy(Path::new(&path), &tmpfile.path())?;

        Ok(tmpfile)
    }

    /// create a new example phenopacket from data/phenopacket.pb
    pub fn example_phenopacket() -> Result<Phenopacket, PhenoStoreError> {
        read_phenopacket(Path::new("data/phenopacket.pb"))
    }

    /// create an empty example store in a temporary directory
    /// the TempDir needs to be returned and kept alive a long as you want to use the store, otherwise the dir is removed.
    pub fn example_store_empty() -> Result<FilePhenoStore<tempfile::TempDir>, PhenoStoreError> {
        let temp_dir = tempfile::TempDir::new()?;
        assert!(temp_dir.path().exists(), "failed to create temp dir");
        let ps = FilePhenoStore::new(temp_dir)?;
        Ok(ps)
    }
}

#[cfg(test)]
mod test_core {
    use std::path::Path;

    use crate::core::*;

    #[test]
    fn test_read_phenopacket() {
        let path = Path::new("data/phenopacket.pb");
        let pp = read_phenopacket(&path);

        assert!(pp.is_ok());
        let pp = pp.unwrap();
        assert_eq!(pp.id, "comprehensive-phenopacket-id");
    }

    #[test]
    fn test_read_create_phenopacket() -> Result<(), PhenoStoreError> {
        let path = Path::new("data/phenopacket.pb");
        let pp = read_phenopacket(&path);

        assert!(pp.is_ok());
        let pp_read = pp.unwrap();
        assert_eq!(pp_read.id, "comprehensive-phenopacket-id");

        // write
        let outfile = tempfile::NamedTempFile::new()?;
        write_phenopacket(&pp_read, &outfile.path())?;
        assert!(!std::fs::metadata(outfile.path())?.len() > 0);

        // read again to check persistence
        let pp_created = read_phenopacket(&outfile.path())?;
        assert_eq!(
            pp_read, pp_created,
            "the written phenopacket does not match the read phenopacket."
        );

        Ok(())
    }

    #[test]
    fn test_delete_phenopacket() -> Result<(), PhenoStoreError> {
        let deleteme_file = testutils::tmpfilecopy_from("data/phenopacket.pb")?;
        assert!(
            &deleteme_file.path().exists(),
            "the test dummy file should exist here, but does not."
        );

        delete_phenopacket(&deleteme_file.path())?;

        assert!(
            !&deleteme_file.path().exists(),
            "the test dummy file should not exist anymore, but does."
        );
        Ok(())
    }

    #[test]
    fn test_update_phenopacket() -> Result<(), PhenoStoreError> {
        let pp_path = "data/phenopacket.pb";
        let file = testutils::tmpfilecopy_from(pp_path)?;
        let mut pp = read_phenopacket(&file.path())?;

        // change some values
        pp.id = "a new id".into();

        // apply changes
        replace_phenopacket(&pp, &file.path())?;

        // verify
        assert!(&file.path().exists());
        let pp_orig = read_phenopacket(Path::new(pp_path))?;
        let pp_updated = read_phenopacket(&file.path())?;
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
    use crate::core::*;
    use std::{assert_matches, fs};

    #[test]
    fn test_new() -> Result<(), PhenoStoreError> {
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
    fn test_new_fail_not_empty() -> Result<(), PhenoStoreError> {
        let temp_dir = tempfile::TempDir::new()?;
        let store_path = temp_dir.path().join("store");
        fs::create_dir(&store_path)?;
        let _ = File::create(store_path.join("dummyfile.pb"))?;
        assert!(
            store_path.join("dummyfile.pb").exists(),
            "sth went wrong during test setup"
        );

        assert!(
            FilePhenoStore::new(&store_path).is_err(),
            "creating an empty store in a non-empty dir should fail."
        );
        Ok(())
    }

    #[test]
    fn test_open() -> Result<(), PhenoStoreError> {
        let temp_dir = tempfile::TempDir::new()?;
        let ps = FilePhenoStore::open(temp_dir.path())?;
        assert_eq!(
            temp_dir.path(),
            ps.dir,
            "failed to set the store directory."
        );
        Ok(())
    }

    #[test]
    fn test_add() -> Result<(), PhenoStoreError> {
        let temp_dir = tempfile::TempDir::new()?;
        let ps = FilePhenoStore::new(temp_dir.path())?;
        let pp = testutils::example_phenopacket()?;
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
    fn test_remove() -> Result<(), PhenoStoreError> {
        // setup
        let ps = testutils::example_store_empty()?;

        // successfully remove a stored phenopacket
        let pp = testutils::example_phenopacket()?;
        let id = ps.add(&pp)?;
        let test_exists = ps.remove(&id);
        assert_matches!(test_exists, Ok(_));
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
        assert_matches!(test_nonexists, Err(PhenoStoreError::NotFound));
        Ok(())
    }

    #[test]
    fn test_get() -> Result<(), PhenoStoreError> {
        // setup
        let temp_dir = tempfile::TempDir::new()?;
        let ps = FilePhenoStore::new(temp_dir.path())?;

        // successfully get a stored phenopacket
        let pp = testutils::example_phenopacket()?;
        let id = ps.add(&pp)?;
        let test_exists = ps.get(&id);
        assert_matches!(test_exists, Ok(Some(_)));

        // valid id, non-existend in store
        let test_notfound = ps.get(&Uuid::new_v4());
        assert_matches!(test_notfound, Ok(None));

        // valid id and file name, but corrupted contents (not valid phenopacket)
        let id_corrupted_file = Uuid::new_v4();
        let mut corrupted_file = fs::File::create_new(ps.get_phenopacket_path(&id_corrupted_file))?;
        let _ = write!(
            corrupted_file,
            "ohno this file is corrupted and does not contain valid pb data :("
        );
        let test_fail = ps.get(&id_corrupted_file);
        assert_matches!(
            test_fail,
            Err(PhenoStoreError::Storage(StorageError::Corrupted))
        );

        Ok(())
    }

    #[test]
    fn test_len() -> Result<(), PhenoStoreError> {
        let ps = testutils::example_store_empty().expect("failed to set up test pheno store");
        let len0 = ps.len()?;

        let pp1 = testutils::example_phenopacket()?;
        let _ = ps.add(&pp1)?;
        let len1 = ps.len()?;

        let pp2 = testutils::example_phenopacket()?;
        let _ = ps.add(&pp2)?;
        let len2 = ps.len()?;

        assert_eq!(len0, 0);
        assert_eq!(len1, 1);
        assert_eq!(len2, 2);
        Ok(())
    }
}
