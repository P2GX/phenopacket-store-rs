use std::cmp::Ordering;
use std::time::Duration;

use phenopackets::{
    ga4gh::vrsatile::v1::GeneDescriptor,
    schema::v2::{
        Phenopacket,
        core::{Sex, genomic_interpretation::Call, vital_status::Status},
    },
};

#[cfg(feature = "ontology-query")]
use ontolius::TermId;
#[cfg(feature = "ontology-query")]
use ontolius::ontology::TaxonomyQuery;

use crate::{impl_bool_algebra, query::Query};

#[derive(Clone)]
/// check whether a patient is diagnosed with a certain disease.
///
/// # Fields
/// - (`String`) - The disease id.
// TODO: add an example disease id
pub struct DiagnosedWith(pub String);
impl_bool_algebra!(DiagnosedWith);
impl Query<Phenopacket> for DiagnosedWith {
    fn test(&self, item: &Phenopacket) -> bool {
        item.diseases
            .iter()
            .any(|disease| disease.term.as_ref().is_some_and(|term| term.id == self.0))
    }
}

#[derive(Clone)]
//ASK double check phrasing
/// Test if the phenopacket has a genomic interpretation with either gene context corresponding to the query
/// or a variant interpretation with the gene context corresponding to the query.
///
/// # Fields
/// - (`String`) - the query gene id (e.g. `"HGNC:3603"` for *FBN1*).
// TODO: add an example HGNC gene id
pub struct HasVariantInGene(pub String);
impl HasVariantInGene {
    fn matches_gene_descr(&self, gd: &GeneDescriptor) -> bool {
        gd.value_id == self.0 || gd.alternate_ids.contains(&self.0)
    }
}
impl_bool_algebra!(HasVariantInGene);
impl Query<Phenopacket> for HasVariantInGene {
    fn test(&self, item: &Phenopacket) -> bool {
        item.interpretations
            .iter()
            .filter_map(|i| i.diagnosis.as_ref())
            .flat_map(|diagnosis| diagnosis.genomic_interpretations.iter())
            .any(|gi| match &gi.call {
                Some(Call::VariantInterpretation(vi)) => vi
                    .variation_descriptor
                    .as_ref()
                    .and_then(|vd| vd.gene_context.as_ref())
                    .is_some_and(|gd| self.matches_gene_descr(gd)),
                Some(Call::Gene(gd)) => self.matches_gene_descr(gd),
                None => false,
            })
    }
}

/// filter by age.
///
/// # Fields
/// - ([`std::cmp::Ordering`]) - enum specifying whether the tested phenopacket's age needs to be smaller, greater or equal to the given age
/// - (u64) - the age to compare the phenopacket to
///
/// a query, testing if a phenopacket's subject is older than 40 could look like this: `Age(sdt::cmp::Ordering::Greater, 40).test(&my_phenopacket)`
// TODO: // The query should operate on the `time_at_last_encounter` (the Age variant only), not on the `date_of_birth`.
//
// Moreover, we must provide more convenience here, to simplify creating the age query
// from different input formats (years, months, years, months, days).
// We should provide at least one "static constructor" that takes the number of years, months, and days as mandatory unsigned values of a reasonable type.
//
// Last, we should add examples for the constructor. No need to actually call the `test` method.
//
// BTW, how do we select phenopackets with individuals less than or equal to, say, 40 years of age?
#[derive(Clone)]
pub struct Age(pub Ordering, pub u64); // make the fields private here
impl_bool_algebra!(Age);
impl Query<Phenopacket> for Age {
    fn test(&self, item: &Phenopacket) -> bool {
        let age = item
            .subject
            .as_ref()
            .and_then(|x| {
                x.date_of_birth.map(|birthday| {
                    let today = std::time::SystemTime::now();
                    let today_unix = today
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .expect("this should be well tested and not fail.");
                    let birthday_unix = Duration::from_secs(birthday.seconds.unsigned_abs());
                    let age_in_seconds = (today_unix - birthday_unix).as_secs();
                    let age_in_years = age_in_seconds / 31556952;
                    age_in_years
                })
            })
            .unwrap_or_default();

        match self.0 {
            Ordering::Less => age < self.1,
            Ordering::Equal => age == self.1,
            Ordering::Greater => age > self.1,
        }
    }
}

/// does the subject have the specified vital [`Status`]?
// TODO: provide convenience constructors for "is_alive", "is_deceased", "is_unknown".
#[derive(Clone)]
pub struct HasVitalStatus(pub Status); // make this private.
impl_bool_algebra!(HasVitalStatus);
impl Query<Phenopacket> for HasVitalStatus {
    fn test(&self, item: &Phenopacket) -> bool {
        item.subject
            .as_ref()
            .is_some_and(|i| match i.vital_status.as_ref() {
                Some(vs) => vs.status() == self.0,
                None => Status::UnknownStatus == self.0,
            })
    }
}

/// does the phenopacket have the specified [`Sex`]?
#[derive(Clone)]
pub struct IsSex(pub Sex);
impl_bool_algebra!(IsSex);
impl Query<Phenopacket> for IsSex {
    fn test(&self, item: &Phenopacket) -> bool {
        item.subject
            .as_ref()
            .map_or(Sex::UnknownSex, |individual| individual.sex())
            == self.0
    }
}

//TODO recurse down with ontolius dependency
// check if t.id is child of self.hpo_term
/// is the phenopacket annotated with the specified `hpo_term` (included or excluded)?
// TODO: this is actually incorrect at the moment.
//  A present feature (`excluded=false`) implies presence of all its ancestors
//  but an excluded feature (`excluded=true`) implies exclusion of all its *descendants*.
#[cfg(feature = "ontology-query")]
#[derive(Clone)]
pub struct HasPhenotypicFeature<T> {
    pub term_id: ontolius::TermId,
    pub excluded: bool,
    pub taxonomy: T,
}

#[cfg(feature = "ontology-query")]
impl_bool_algebra!(HasPhenotypicFeature<T>);

#[cfg(feature = "ontology-query")]
impl<T> Query<Phenopacket> for HasPhenotypicFeature<T>
where
    T: TaxonomyQuery,
{
    fn test(&self, item: &Phenopacket) -> bool {
        item.phenotypic_features.iter().any(|pf| {
            pf.excluded == self.excluded
                && pf.r#type.as_ref().is_some_and(|t| {
                    let item_term_id: TermId = t.id.parse().unwrap(); //TODO get rid of unwrap and document the behavior.

                    item_term_id == self.term_id
                        || self.taxonomy.is_descendant_of(&item_term_id, &self.term_id)
                })
        })
    }
}

//
// TESTS
//
#[cfg(test)]
mod test_queries {
    use super::{DiagnosedWith, HasVariantInGene};
    use crate::{
        phenopacket::fs::testutils,
        query::{
            Query,
            phenopacket::{Age, HasVitalStatus, IsSex},
        },
    };
    use ontolius::TermId;
    use phenopackets::schema::v2::core::{Sex, vital_status::Status};
    use std::cmp::Ordering;

    #[test]
    fn test_diagnosed_with() {
        // TODO: create a `cfg(test)` (see an example at https://github.com/ielis/ontolius/blob/fe10ac2e3a22e6e3c5482a19c91825ba8045b0dc/src/lib.rs#L11)
        // or create module-scoped test util modules instead of using the `testutil` module belonging to another functionality.
        let pp = testutils::example_phenopacket();
        let has_pfeiffer_syndrom = DiagnosedWith(String::from("OMIM:101600"));
        let has_foo = DiagnosedWith(String::from("OMIM:FOOOOOO"));
        assert!(has_pfeiffer_syndrom.test(&pp));
        assert!(!has_foo.test(&pp));
    }

    #[test]
    fn test_variant_in_gene() {
        let pp = testutils::example_phenopacket();
        // let test_predicate_1 = HasVariantInGene(String::from("HGNC:0000135")); //TODO find passing test cases (vi, gd) in testdata
        let test_predicate_2 = HasVariantInGene(String::from("HGNC:FOOOOOO"));
        // assert!(test_predicate_1.test(&pp));
        assert!(!test_predicate_2.test(&pp));
    }

    #[test]
    fn test_age() {
        let pp = testutils::example_phenopacket();
        let test_gt_10 = Age(Ordering::Greater, 10);
        let test_gt_100 = Age(Ordering::Greater, 100);
        let test_le_3 = Age(Ordering::Less, 3);
        assert!(test_gt_10.test(&pp));
        assert!(!test_gt_100.test(&pp));
        assert!(!test_le_3.test(&pp));
    }

    #[test]
    fn test_vital_status() {
        let pp = testutils::example_phenopacket();
        let alive = HasVitalStatus(Status::Alive);
        let dead = HasVitalStatus(Status::Deceased);
        let unknown_vital_status = HasVitalStatus(Status::UnknownStatus);

        assert!(
            !(dead | alive).test(&pp),
            "The test phenopacket has no vital status annotated."
        );
        assert!(
            unknown_vital_status.test(&pp),
            "The test phenopacket has no vital status annotated."
        );
    }

    #[test]
    fn test_is_sex() {
        let pp = testutils::example_phenopacket();

        assert!(
            IsSex(Sex::Male).test(&pp),
            "example phenopacket is annotated as MALE"
        );
        assert!(
            !IsSex(Sex::OtherSex).test(&pp),
            "example phenopacket is annotated as MALE"
        );
    }

    #[cfg(feature = "ontology-query")]
    #[test]
    fn test_annotated_with_hpo() {
        use crate::query::phenopacket::HasPhenotypicFeature;
        use ontolius::ontology::csr::MinimalCsrOntology;

        let pp = testutils::example_phenopacket();
        let hpo: MinimalCsrOntology = {
            use ontolius::io::OntologyLoaderBuilder;
            use std::{fs::File, io::BufReader, path::Path};

            let path = Path::new("data").join("hp.json");
            let loader = OntologyLoaderBuilder::new().obographs_parser().build();
            let reader =
                BufReader::new(File::open(path).expect("setup: failed to open ontology file."));

            loader
                .load_from_buf_read(reader)
                .expect("failed to load from reader.")
        };

        assert!(
            HasPhenotypicFeature {
                term_id: "HP:0001270".parse().unwrap(),
                excluded: false,
                taxonomy: &hpo,
            }
            .test(&pp)
        );
        assert!(
            !HasPhenotypicFeature {
                term_id: "HP:0001270".parse().unwrap(),
                excluded: true,
                taxonomy: &hpo,
            }
            .test(&pp)
        );
        assert!(
            !HasPhenotypicFeature {
                term_id: "HP:FOOOOOO".parse().unwrap(),
                excluded: false,
                taxonomy: &hpo,
            }
            .test(&pp)
        );

        // test mechanics with ancestor / decendent terms
        // t1 -> q -> t2
        // q_excl is an excluded term contained in the example phenopacket
        let q_excl: TermId = "HP:0031910".parse().unwrap(); // Abn. cranial nerve physiology
        // t1 is parent of p
        let q_par: TermId = "HP:0012638".parse().unwrap(); // Abn. nervous system physiology
        // t2 is child of p
        let q_child: TermId = "HP:0006824".parse().unwrap(); // Cranial nerve paralysis

        let query = HasPhenotypicFeature {
            term_id: q_excl.clone(),
            excluded: true,
            taxonomy: &hpo,
        };
        assert!(query.test(&pp));

        let query = HasPhenotypicFeature {
            term_id: q_par.clone(),
            excluded: true,
            taxonomy: &hpo,
        };
        assert!(
            !query.test(&pp),
            "exclusion of a term does NOT exclude any of its ancestors."
        );

        let query = HasPhenotypicFeature {
            term_id: q_child.clone(),
            excluded: true,
            taxonomy: &hpo,
        };
        assert!(
            !query.test(&pp),
            "exclusion of a term also excludes its descendants"
        );

        // test the exclusion
        let query = HasPhenotypicFeature {
            term_id: q_excl.clone(),
            excluded: false,
            taxonomy: &hpo,
        };
        assert!(!query.test(&pp), "excluded term is not present");
    }
}
