use std::io::{BufRead, Write};

use super::Cohort;

/// Implementors support reading/writing of a [`Cohort`] from/to IO streams
/// in a particular data format (e.g. JSON).
pub trait CohortCodec {
    type Error;

    /// Read the cohort from a reader.
    fn read<R>(&self, r: &mut R) -> Result<Cohort, Self::Error>
    where
        R: BufRead;

    /// Write the cohort into a writer.
    fn write<W>(&self, c: &Cohort, w: &mut W) -> Result<(), Self::Error>
    where
        W: Write;

    /// Returns the file format extention for the cohort files
    /// (e.g. `json` for the JSON format).
    fn ext(&self) -> &str;
}

impl<T> CohortCodec for &T
where
    T: CohortCodec + ?Sized,
{
    type Error = T::Error;

    fn read<R: BufRead>(&self, r: &mut R) -> Result<Cohort, Self::Error> {
        (*self).read(r)
    }

    fn write<W: Write>(&self, c: &Cohort, w: &mut W) -> Result<(), Self::Error> {
        (*self).write(c, w)
    }

    fn ext(&self) -> &str {
        (*self).ext()
    }
}

impl<T> CohortCodec for Box<T>
where
    T: CohortCodec + ?Sized,
{
    type Error = T::Error;

    fn read<R: BufRead>(&self, r: &mut R) -> Result<Cohort, Self::Error> {
        (**self).read(r)
    }

    fn write<W: Write>(&self, c: &Cohort, w: &mut W) -> Result<(), Self::Error> {
        (**self).write(c, w)
    }

    fn ext(&self) -> &str {
        (**self).ext()
    }
}

/// implements [`CohortCodec`] using [`serde_json`].
pub struct JsonCohortCodec;

impl CohortCodec for JsonCohortCodec {
    type Error = serde_json::Error;

    fn read<R: BufRead>(&self, r: &mut R) -> Result<Cohort, Self::Error> {
        let c: Cohort = serde_json::from_reader(r)?;
        Ok(c)
    }

    fn write<W: Write>(&self, c: &Cohort, w: &mut W) -> Result<(), Self::Error> {
        serde_json::to_writer(w, c)?;
        Ok(())
    }

    fn ext(&self) -> &str {
        "json"
    }
}
