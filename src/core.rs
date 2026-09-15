use std::{error::Error, fs::{self, File}, io::{Read, Write}, path::Path};

use phenopackets::schema::v2::Phenopacket;
use prost::Message;

/// read a phenopacket from the specified path
pub fn read_phenopacket(path: &Path) -> Result<Phenopacket, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(Phenopacket::decode(&buf[..])?)
}

/// create a new phenopacket at the specified path
pub fn create_phenopacket(phenopacket: &Phenopacket, path: &Path) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let mut buf = Vec::new();
    phenopacket.encode(&mut buf)?;
    file.write_all(&buf)?;

    Ok(())
}

/// delete a phenopacket from the specified path
pub fn delete_phenopacket(path: &Path) -> std::io::Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

/// write a phenopacket to the specified path
pub fn update_phenopacket(phenopacket: &Phenopacket, path: &Path) -> Result<(), Box<dyn Error>> {
    delete_phenopacket(path)?;
    create_phenopacket(phenopacket, path)?;

    Ok(())
}



//
// TESTS
//
#[cfg(test)]
mod test_core {
    use std::{fs, path::Path, error::Error};

    use crate::core::{delete_phenopacket, read_phenopacket, update_phenopacket, create_phenopacket};
    
    /// get a temporary copy of the provided file. the copy will be deleted once the filehandle is dropped.
    fn tmpfilecopy_from(path: &str) -> std::io::Result<tempfile::NamedTempFile> {
        let tmpfile = tempfile::NamedTempFile::new()?;
        fs::copy(Path::new(&path), &tmpfile.path())?;

        Ok(tmpfile)
    }

    #[test]
    fn test_read_phenopacket() {
        let path = Path::new("data/phenopacket.pb");
        let pp = read_phenopacket(&path);

        assert!(pp.is_ok());
        let pp = pp.unwrap();
        assert_eq!(pp.id, "comprehensive-phenopacket-id");
    }

    #[test]
    fn test_read_create_phenopacket() -> Result<(), Box<dyn Error>> {
        let path = Path::new("data/phenopacket.pb");
        let pp = read_phenopacket(&path);

        assert!(pp.is_ok());
        let pp_read = pp.unwrap();
        assert_eq!(pp_read.id, "comprehensive-phenopacket-id");

        // write
        let outfile = tempfile::NamedTempFile::new()?;
        create_phenopacket(&pp_read, &outfile.path())?;
        assert!(!std::fs::metadata(outfile.path())?.len() > 0);

        // read again to check persistence
        let pp_created = read_phenopacket(&outfile.path())?;
        assert_eq!(pp_read, pp_created, "the written phenopacket does not match the read phenopacket.");

        Ok(())
    }

    #[test]
    fn test_delete_phenopacket_success() -> Result<(), Box<dyn Error>>{
        // let deleteme_file = tempfile::NamedTempFile::new()?;
        // let deleteme_path = deleteme_file.path();
        // fs::copy(Path::new("data/phenopacket.pb"), &deleteme_path)?;
        let deleteme_file = tmpfilecopy_from("data/phenopacket.pb")?;
        assert!(&deleteme_file.path().exists(), "the test dummy file should exist here, but does not.");

        delete_phenopacket(&deleteme_file.path())?;

        assert!(!&deleteme_file.path().exists(), "the test dummy file should not exist anymore, but does.");
        Ok(())
    }

    #[test]
    fn test_update_phenopacket_success() -> Result<(), Box<dyn Error>>{
        let pp_path = "data/phenopacket.pb";
        let file = tmpfilecopy_from(pp_path)?;
        let mut pp = read_phenopacket(&file.path())?;

        // change some values
        pp.id = "a new id".into();
        
        // apply changes
        update_phenopacket(&pp, &file.path())?;
        
        // verify
        assert!(&file.path().exists());
        let pp_orig = read_phenopacket(Path::new(pp_path))?;
        let pp_updated = read_phenopacket(&file.path())?;
        assert_ne!(pp_orig, pp_updated, "no changes were applied to the stored phenopacket");
        assert_eq!(pp.id, pp_updated.id);
        Ok(())
    }
}
