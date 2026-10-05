use std::path::PathBuf;

use psrepo::{PhenoStore, fs::FilePhenoStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ps = configure()?;
    let a = &*ps;
    use_pheno_store(a);
    Ok(())
}

fn configure() -> Result<Box<impl PhenoStore>, Box<dyn std::error::Error>> {
    let cfg = "db";
    match cfg {
        "db" => todo!(),
        "file" => Ok(Box::new(FilePhenoStore::new(PathBuf::from("path/to/dir"))?)),
        _ => todo!(),
    }
}

fn use_pheno_store<T>(ps: T)
where
    T: PhenoStore,
{
    ps.add(todo!()).unwrap();
    ps.get(todo!()).unwrap();
}
