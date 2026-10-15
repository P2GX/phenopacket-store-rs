use std::time::Duration;

use phenopackets::{
    ga4gh::vrsatile::v1::GeneDescriptor,
    schema::v2::{
        Phenopacket,
        core::{Sex, genomic_interpretation::Call, vital_status::Status},
    },
};

use crate::{impl_bool_algebra, query::Query};

#[derive(Clone)]
/// check whether a patient is diagnosed with a certain disease.
///
/// # Fields
/// - (`String`) - The disease id.
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
/// check whether a gene contains variants or is somewhat significant.
///
/// # Fields
/// - (`String`) - the gene id.
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
#[derive(Clone)]
pub struct Age(std::cmp::Ordering, u64);
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
            std::cmp::Ordering::Less => age < self.1,
            std::cmp::Ordering::Equal => age == self.1,
            std::cmp::Ordering::Greater => age > self.1,
        }
    }
}

/// does the subject have the specified vital [`Status`]?
#[derive(Clone)]
pub struct HasVitalStatus(Status);
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
pub struct IsSex(Sex);
impl_bool_algebra!(IsSex);
impl Query<Phenopacket> for IsSex {
    fn test(&self, item: &Phenopacket) -> bool {
        item.subject
            .as_ref()
            .map_or(Sex::UnknownSex, |individual| individual.sex())
            == self.0
    }
}

//TODO should we check for evidence aswell?
/// is phenotype annotated with an HPO term without severity, modifiers, onset or resolution?
#[derive(Clone)]
pub struct HasHpoLackingContext;
impl_bool_algebra!(HasHpoLackingContext);
impl Query<Phenopacket> for HasHpoLackingContext {
    fn test(&self, item: &Phenopacket) -> bool {
        item.phenotypic_features.iter().any(|pf| {
            pf.severity.is_none()
                && pf.modifiers.is_empty()
                && pf.onset.is_none()
                && pf.onset.is_none()
                && pf.resolution.is_none()
        })
    }
}

/// is the phenopacket annotated with the specified `hpo_term` (included or excluded)?
#[derive(Clone)]
pub struct AnnotatedWithHPO {
    hpo_term: String,
    excluded: bool,
}
impl_bool_algebra!(AnnotatedWithHPO);
impl Query<Phenopacket> for AnnotatedWithHPO {
    fn test(&self, item: &Phenopacket) -> bool {
        item.phenotypic_features.iter().any(|pf| {
            pf.r#type.as_ref().is_some_and(|t| t.id == self.hpo_term)
                && pf.excluded == self.excluded
        })
    }
}

//
// TESTS
//
#[cfg(test)]
mod test_queries {
    use phenopackets::schema::v2::core::{Sex, vital_status::Status};
    use std::cmp::Ordering;

    use super::{DiagnosedWith, HasVariantInGene};
    use crate::{
        phenopacket::{
            fs::testutils,
            query::{Age, HasVitalStatus, IsSex},
        },
        query::Query,
    };

    #[test]
    fn test_diagnosed_with() {
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
}
