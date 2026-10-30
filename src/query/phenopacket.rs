use phenopackets::{
    ga4gh::vrsatile::v1::GeneDescriptor,
    schema::v2::{
        Phenopacket,
        core::{Sex, genomic_interpretation::Call, vital_status::Status},
    },
};

#[cfg(feature = "ontolius")]
use ontolius::TermId;
#[cfg(feature = "ontolius")]
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

/// Test if the phenopacket has a genomic interpretation with variation descriptor annotated to the query gene.
///
/// Checks the [`gene_context`](https://phenopacket-schema.readthedocs.io/en/latest/variant.html#data-model) field of the [VariationDescriptor](https://phenopacket-schema.readthedocs.io/en/latest/variant.html#rstvariant).
///
/// # Fields
/// - (`String`) - the query gene id (e.g. `"HGNC:3603"` for *FBN1*).
///
/// # Examples
///
/// We can test whether our patient has a genomic interpretation concerning fibrillin 1.
///
/// ```
/// use ppktstore::query::phenopacket::HasVariantInGene;
///
/// let query = HasVariantInGene(String::from("HGNC:3603"));  // FBN1
/// ```
#[derive(Debug, Clone)]
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
            .any(|gi| {
                if let Some(Call::VariantInterpretation(vi)) = &gi.call {
                    vi.variation_descriptor
                        .as_ref()
                        .and_then(|vd| vd.gene_context.as_ref())
                        .is_some_and(|gd| self.matches_gene_descr(gd))
                } else {
                    false
                }
            })
    }
}

/// does the subject have the specified vital [`Status`]?
#[derive(Debug, Clone)]
pub struct HasVitalStatus(Status);

impl HasVitalStatus {
    /// Test if the phenopacket represents a living individual.
    pub fn is_alive() -> Self {
        Self(Status::Alive)
    }

    /// Test if the phenopacket represents a deceased individual.
    pub fn is_deceased() -> Self {
        Self(Status::Deceased)
    }

    /// Test if no vital status is indicated by the phenopacket.
    pub fn has_unknown_vital_status() -> Self {
        Self(Status::UnknownStatus)
    }
}

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

/// Test if the phenopacket represents an individual with specified sex.
#[derive(Debug, Clone)]
pub struct HasSex(Sex);
impl HasSex {
    pub fn is_male() -> Self {
        HasSex(Sex::Male)
    }

    pub fn is_female() -> Self {
        HasSex(Sex::Female)
    }

    pub fn has_unknown_sex() -> Self {
        HasSex(Sex::UnknownSex)
    }

    pub fn has_other_sex() -> Self {
        HasSex(Sex::OtherSex)
    }
}

impl_bool_algebra!(HasSex);
impl Query<Phenopacket> for HasSex {
    fn test(&self, item: &Phenopacket) -> bool {
        item.subject
            .as_ref()
            .map_or(Sex::UnknownSex, |individual| individual.sex())
            == self.0
    }
}

/// Test if the phenopacket is annotated to an HPO `query` in given observation state (present or excluded).
///
/// The phenopacket must contain no logical contradictions, such as term in both present and excluded observation states
/// or a present term and its excluded ancestor.
///
/// The query does *not* take temporal aspects into the account!
///
/// # Fields
/// query - the query term id
/// excluded - the observation state of the query (`false` for a present term and `true` for a term that was sought for but found to be absent)
/// taxonomy - the taxonomy for navigating through ontology hierarchy.
///
/// check [`ontolius`] docs on how to load a taxonomy.
#[cfg(feature = "ontolius")]
#[derive(Debug, Clone)]
pub struct HasPhenotypicFeature<T> {
    pub query: ontolius::TermId,
    /// is this phenotypic feature *explicitly excluded*? for example because it was explicitly looked for, but found to be absent.
    pub excluded: bool,
    pub taxonomy: T,
}

#[cfg(feature = "ontolius")]
impl_bool_algebra!(HasPhenotypicFeature<T>);

#[cfg(feature = "ontolius")]
impl<T> Query<Phenopacket> for HasPhenotypicFeature<T>
where
    T: TaxonomyQuery,
{
    fn test(&self, item: &Phenopacket) -> bool {
        item.phenotypic_features
            .iter()
            .filter(|pf| pf.excluded == self.excluded)
            .any(|current_pf| {
                if let Some(item_term_id) = current_pf
                    .r#type
                    .as_ref()
                    .and_then(|t| t.id.parse::<TermId>().ok())
                {
                    if !self.excluded {
                        self.taxonomy
                            .is_equal_or_ancestor_of(&self.query, &item_term_id)
                    } else {
                        self.taxonomy
                            .is_equal_or_ancestor_of(&item_term_id, &self.query)
                    }
                } else {
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
        query::{
            Query,
            phenopacket::{DiagnosedWith, HasSex, HasVariantInGene, HasVitalStatus},
        },
        testutils,
    };
    use ontolius::TermId;

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
        let test_col6a1 = HasVariantInGene(String::from("HGNC:2211"));
        let test_nonsense_gene = HasVariantInGene(String::from("HGNC:FOOOOOO"));
        assert!(
            test_col6a1.test(&pp),
            "variant interpretation concerning COL6A1 is contained in example pp and shoud be found."
        );
        assert!(!test_nonsense_gene.test(&pp));
    }

    #[test]
    fn test_vital_status() {
        let pp = testutils::example_phenopacket();
        let alive = HasVitalStatus::is_alive();
        let dead = HasVitalStatus::is_deceased();
        let unknown_vital_status = HasVitalStatus::has_unknown_vital_status();

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
            HasSex::is_male().test(&pp),
            "example phenopacket is annotated as MALE"
        );
        assert!(
            !HasSex::has_other_sex().test(&pp),
            "example phenopacket is annotated as MALE"
        );
    }

    #[cfg(feature = "ontolius")]
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
                query: q_incl.clone(),
                excluded: false,
                taxonomy: &hpo
            }
            .test(&pp),
            "inclusion query for exact matching terms should work."
        );
        assert!(
            HasPhenotypicFeature {
                query: q_par.clone(),
                excluded: false,
                taxonomy: &hpo
            }
            .test(&pp),
            "inclusion of a term also includes its parents."
        );
        assert!(
            !HasPhenotypicFeature {
                query: q_child.clone(),
                excluded: false,
                taxonomy: &hpo
            }
            .test(&pp),
            "inclusion of a term does NOT include its children."
        );
        assert!(
            HasPhenotypicFeature {
                query: q_par2.clone(),
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
            query: q_excl.clone(),
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
            query: q_par.clone(),
            excluded: true,
            taxonomy: &hpo,
        };
        assert!(
            !query.test(&pp),
            "exclusion of a term does NOT exclude any of its ancestors."
        );

        let query = HasPhenotypicFeature {
            query: q_child.clone(),
            excluded: true,
            taxonomy: &hpo,
        };
        assert!(
            query.test(&pp),
            "exclusion of a term also excludes its descendants"
        );

        // test the exclusion
        let query = HasPhenotypicFeature {
            query: q_excl.clone(),
            excluded: false,
            taxonomy: &hpo,
        };
        assert!(!query.test(&pp), "excluded term is not present");
    }
}
