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

#[derive(Debug, Clone)]
/// check whether a patient is diagnosed with a certain disease.
///
/// # Fields
/// - (`String`) - The disease id.
///
/// # Examples
///
/// We can test, whether our patient was diagnosed with pfeiffers syndrome. In this example we would use the OMIM ontology to annotate mendelian diseases.
///
/// ```
/// use ppktstore::query::phenopacket::DiagnosedWith;
///
/// let has_pfeiffer_syndrome = DiagnosedWith(String::from("OMIM:101600"));
/// ```
pub struct DiagnosedWith(pub String);
impl_bool_algebra!(DiagnosedWith);
impl Query<Phenopacket> for DiagnosedWith {
    fn test(&self, item: &Phenopacket) -> bool {
        item.diseases
            .iter()
            .any(|disease| disease.term.as_ref().is_some_and(|term| term.id == self.0))
    }
}

#[derive(Debug, Clone)]
//ASK double check phrasing
/// Test if the phenopacket has a genomic interpretation with either gene context corresponding to the query
/// or a variant interpretation with the gene context corresponding to the query.
///
/// # Fields
/// - (`String`) - the query gene id (e.g. `"HGNC:3603"` for *FBN1*).
///
/// # Examples
///
/// We can test whether our patient has a genomic interpretation concerning some gene.
///
/// ```
/// use ppktstore::query::phenopacket::HasVariantInGene;
///
/// let FBN1_gene = String::from("NCBIGene:2200");
/// let query_FBN1 = HasVariantInGene(FBN1_gene);
/// ```
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

/// does the subject have the specified vital [`Status`]?
#[derive(Debug, Clone)]
struct HasVitalStatus(Status);
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

/// is the subject alive?
#[derive(Debug, Clone)]
pub struct IsAlive;
impl_bool_algebra!(IsAlive);
impl Query<Phenopacket> for IsAlive {
    fn test(&self, item: &Phenopacket) -> bool {
        HasVitalStatus(Status::Alive).test(item)
    }
}

/// is the subject deceased?
#[derive(Debug, Clone)]
pub struct IsDeceased;
impl_bool_algebra!(IsDeceased);
impl Query<Phenopacket> for IsDeceased {
    fn test(&self, item: &Phenopacket) -> bool {
        HasVitalStatus(Status::Deceased).test(item)
    }
}

/// is the subjects vital staus unknown?
#[derive(Debug, Clone)]
pub struct IsUnknownStatus;
impl_bool_algebra!(IsUnknownStatus);
impl Query<Phenopacket> for IsUnknownStatus {
    fn test(&self, item: &Phenopacket) -> bool {
        HasVitalStatus(Status::UnknownStatus).test(item)
    }
}

/// does the phenopacket have the specified [`Sex`]?
#[derive(Debug, Clone)]
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

/// is the phenopacket annotated with the specified `hpo_term` (included or excluded)?
///
/// # Fields
/// term_id - ([`TermId`]) - the id of the term you want to query
/// excluded - (`bool`) - is the term excluded in this phenopacket?
/// taxonomy - the taxonomy the term is from / the phenopacket uses. this will be used to check whether the term is implicitly included/excluded by ancestor/decendant terms.
///
/// check [`ontolius`] docs on how to load a taxonomy.
#[cfg(feature = "ontology-query")]
#[derive(Debug, Clone)]
pub struct HasPhenotypicFeature<T> {
    pub term_id: ontolius::TermId,
    /// is this phenotypic feature *explicitly excluded*? for example because it was explicitly looked for, but found to be absent.
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
        item.phenotypic_features.iter().any(|current_pf| {
            if let Some(item_term_id) = current_pf
                .r#type
                .as_ref()
                .and_then(|t| t.id.parse::<TermId>().ok())
            {
                let term_matches = item_term_id == self.term_id;
                if !self.excluded {
                    // is the queried term (implicitly or explicitly) INCLUDED through the current term?
                    let current_is_decendant_of_query =
                        self.taxonomy.is_descendant_of(&item_term_id, &self.term_id);
                    (term_matches || current_is_decendant_of_query) && !current_pf.excluded
                } else {
                    // is the queried term (implicitly or explicitly) EXCLUDED through the current term?
                    let current_is_ancestor_of_query =
                        self.taxonomy.is_ancestor_of(&item_term_id, &self.term_id);
                    (term_matches || current_is_ancestor_of_query) && current_pf.excluded
                }
            } else {
                // we cannot make a statement about this current term.
                false
            }
        })
    }
}

//
// TESTS
//
#[cfg(test)]
mod test_queries {
    use crate::{
        phenopacket::fs::testutils,
        query::{
            Query,
            phenopacket::{
                DiagnosedWith, HasVariantInGene, HasVitalStatus, IsAlive, IsDeceased, IsSex,
                IsUnknownStatus,
            },
        },
    };
    use ontolius::TermId;
    use phenopackets::schema::v2::core::{Sex, vital_status::Status};

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

        // test convenience wrappers
        assert!(IsUnknownStatus.test(&pp));
        assert!(!IsAlive.test(&pp));
        assert!(!IsDeceased.test(&pp));
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
    fn test_has_phenotypic_feature() {
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

        // test INCLUSION mechanics with ancestor / decendent terms
        // q_par2 -> .. -> q_par -> q_incl -> q_child
        // q_incl is an included term contained in the example phenopacket
        let q_incl: TermId = "HP:0001558".parse().unwrap(); // Decreased fetal movement
        // q_par is parent of q
        let q_par: TermId = "HP:0001557".parse().unwrap(); // Prenatal movement abnormality 
        // q_child is child of q
        let q_child: TermId = "HP:0001989".parse().unwrap(); // Fetal akinesia sequence
        // q_par2 is grandparent of q_par
        let q_par2: TermId = "HP:0034057".parse().unwrap(); // Fetal anomaly 

        assert!(
            HasPhenotypicFeature {
                term_id: q_incl.clone(),
                excluded: false,
                taxonomy: &hpo
            }
            .test(&pp),
            "inclusion query for exact matching terms should work."
        );
        assert!(
            HasPhenotypicFeature {
                term_id: q_par.clone(),
                excluded: false,
                taxonomy: &hpo
            }
            .test(&pp),
            "inclusion of a term also includes its parents."
        );
        assert!(
            !HasPhenotypicFeature {
                term_id: q_child.clone(),
                excluded: false,
                taxonomy: &hpo
            }
            .test(&pp),
            "inclusion of a term does NOT include its children."
        );
        assert!(
            HasPhenotypicFeature {
                term_id: q_par2.clone(),
                excluded: false,
                taxonomy: &hpo
            }
            .test(&pp),
            "inclusion of a term does include all of its ancestors."
        );

        // test EXCLUSION mechanics with ancestor / decendent terms
        // q_par -> q_excl -> q_child
        // q_excl is an excluded term contained in the example phenopacket
        let q_excl: TermId = "HP:0031910".parse().unwrap(); // Abn. cranial nerve physiology
        // q_par is parent of q_excl
        let q_par: TermId = "HP:0012638".parse().unwrap(); // Abn. nervous system physiology
        // q_child is child of q_excl
        let q_child: TermId = "HP:0006824".parse().unwrap(); // Cranial nerve paralysis

        let query = HasPhenotypicFeature {
            term_id: q_excl.clone(),
            excluded: true,
            taxonomy: &hpo,
        };
        let x = query.test(&pp);
        assert!(x);
        assert!(
            query.test(&pp),
            "exclusion query for exact matching excluded term should work."
        );

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
            query.test(&pp),
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
