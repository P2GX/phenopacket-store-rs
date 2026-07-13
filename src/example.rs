use std::{error::Error, fs::File, io::Read, path::Path};

use phenopackets::schema::v2::Phenopacket;
use prost::Message;

pub fn read_phenopacket(path: &Path) -> Result<Phenopacket, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(Phenopacket::decode(&buf[..])?)
}

#[cfg(test)]
mod test_example {
    use std::path::Path;

    use crate::example::read_phenopacket;

    #[test]
    fn test_read_phenopacket() {
        let path = Path::new("data/phenopacket.pb");
        let pp = read_phenopacket(&path);

        assert!(pp.is_ok());
        let pp = pp.unwrap();
        assert_eq!(pp.id, "comprehensive-phenopacket-id");
    }
}
