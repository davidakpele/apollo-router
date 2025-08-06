mod satisfiability;

use std::vec;

use apollo_compiler::schema::ExtendedType;
use apollo_compiler::validation::Valid;

pub use crate::composition::satisfiability::validate_satisfiability;
use crate::error::CompositionError;
use crate::merge::merge_federation_subgraphs;
use crate::merge::MergeFailure;
use crate::merge::MergeSuccess;
use crate::query_graph::build_query_graph;
pub use crate::schema::schema_upgrader::upgrade_subgraphs_if_necessary;
use crate::schema::ValidFederationSchema;
use crate::subgraph::typestate::Expanded;
use crate::subgraph::typestate::Initial;
use crate::subgraph::typestate::Subgraph;
use crate::subgraph::typestate::Upgraded;
use crate::subgraph::typestate::Validated;
pub use crate::supergraph::Merged;
pub use crate::supergraph::Satisfiable;
pub use crate::supergraph::Supergraph;
use crate::ValidFederationSubgraph;
use crate::ValidFederationSubgraphs;

pub fn compose(
    subgraphs: Vec<Subgraph<Initial>>,
) -> Result<Supergraph<Satisfiable>, Vec<CompositionError>> {
    let expanded_subgraphs = expand_subgraphs(subgraphs)?;
    let upgraded_subgraphs = upgrade_subgraphs_if_necessary(expanded_subgraphs)?;
    let validated_subgraphs = validate_subgraphs(upgraded_subgraphs)?;

    pre_merge_validations(&validated_subgraphs)?;
    let supergraph = merge_subgraphs(validated_subgraphs)?;
    post_merge_validations(&supergraph)?;

    validate_satisfiability(supergraph)
}

/// Apollo Federation allow subgraphs to specify partial schemas (i.e. "import" directives through
/// `@link`). This function will update subgraph schemas with all missing federation definitions.
pub fn expand_subgraphs(
    subgraphs: Vec<Subgraph<Initial>>,
) -> Result<Vec<Subgraph<Expanded>>, Vec<CompositionError>> {
    let mut errors: Vec<CompositionError> = vec![];
    let expanded: Vec<Subgraph<Expanded>> = subgraphs
        .into_iter()
        .map(|s| s.expand_links())
        .filter_map(|r| r.map_err(|e| errors.push(e.into())).ok())
        .collect();
    if errors.is_empty() {
        Ok(expanded)
    } else {
        Err(errors)
    }
}

/// Validate subgraph schemas to ensure they satisfy Apollo Federation requirements (e.g. whether
/// `@key` specifies valid `FieldSet`s etc).
pub fn validate_subgraphs(
    subgraphs: Vec<Subgraph<Upgraded>>,
) -> Result<Vec<Subgraph<Validated>>, Vec<CompositionError>> {
    let mut errors: Vec<CompositionError> = vec![];
    let validated: Vec<Subgraph<Validated>> = subgraphs
        .into_iter()
        .map(|s| s.validate())
        .filter_map(|r| r.map_err(|e| errors.push(e.into())).ok())
        .collect();
    if errors.is_empty() {
        Ok(validated)
    } else {
        Err(errors)
    }
}

pub fn pre_merge_validations(
    subgraphs: &[Subgraph<Validated>],
) -> Result<(), Vec<CompositionError>> {
    let mut errors = Vec::new();
    
    // Track subgraphs by their schema string representation
    let mut seen_schemas = std::collections::HashSet::new();
    for subgraph in subgraphs {
        let schema_str = subgraph.schema_string();
        if !seen_schemas.insert(schema_str.clone()) {
            errors.push(CompositionError::TypeDefinitionInvalid {
                message: "Duplicate subgraph schema detected".to_string(),
            });
        }
    }

    // Validate each subgraph's schema
    for subgraph in subgraphs {
        let raw_schema = subgraph.schema().schema().clone();
        match ValidFederationSchema::new(Valid::assume_valid(raw_schema)) {
            Ok(_) => (), 
            Err(e) => {
                errors.push(CompositionError::SubgraphError {
                    subgraph: "Subgraph".to_string(),
                    error: e,
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}


pub fn merge_subgraphs(
    subgraphs: Vec<Subgraph<Validated>>,
) -> Result<Supergraph<Merged>, Vec<CompositionError>> {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    // Convert to federation subgraphs format expected by the merger
    let mut subgraphs_map = BTreeMap::new();
    for subgraph in subgraphs {
        let schema = subgraph.schema().schema().clone();
        match ValidFederationSchema::new(Valid::assume_valid(schema)) {
            Ok(valid_schema) => {
                subgraphs_map.insert(
                    Arc::from("Subgraph"), 
                    ValidFederationSubgraph {
                        name: "Subgraph".to_string(),
                        url: "".to_string(),
                        schema: valid_schema, 
                    },
                );
            }
            Err(e) => {
                return Err(vec![CompositionError::SubgraphError {
                    subgraph: "Subgraph".to_string(),
                    error: e,
                }]);
            }
        }
    }

    // Create ValidFederationSubgraphs using struct 
    let subgraphs_for_merge = ValidFederationSubgraphs {
        subgraphs: subgraphs_map,
    };

    // Perform the actual merge
    match merge_federation_subgraphs(subgraphs_for_merge) {
        Ok(MergeSuccess { schema, .. }) => {
            Ok(Supergraph::<Merged>::new(schema))
        }
        Err(MergeFailure { errors, .. }) => Err(errors
            .into_iter()
            .map(|e| CompositionError::InternalError {
                message: e,
            })
            .collect()),
    }
}

pub fn post_merge_validations(
    supergraph: &Supergraph<Merged>,
) -> Result<(), Vec<CompositionError>> {
    let mut errors = Vec::new();
    let schema = supergraph.schema();

    // Schema validation
    if schema.types.is_empty() {
        errors.push(CompositionError::TypeDefinitionInvalid {
            message: "Empty supergraph schema".to_string(),
        });
    }

    // Root operation validateion
    let schema_def = &schema.schema_definition;
    if schema_def.query.is_none() {
        errors.push(CompositionError::TypeDefinitionInvalid {
            message: "Missing root Query type".to_string(),
        });
    }

    // Schema validation
    if let Err(validation_result) = schema.clone().into_inner().validate() {
        for error in validation_result.errors.iter() {
            errors.push(CompositionError::TypeDefinitionInvalid {
                message: error.to_string(),
            });
        }
    }
    
    // Field resolvability checking
    let federation_schema = match ValidFederationSchema::new(schema.clone()) {
        Ok(s) => s,
        Err(e) => {
            errors.push(CompositionError::InternalError {
                message: format!("Failed to convert schema: {}", e),
            });
            return if errors.is_empty() { Ok(()) } else { Err(errors) };
        }
    };

    // Build query graph
    let _query_graph = match build_query_graph::build_federated_query_graph(
        federation_schema.clone(),
        federation_schema.clone(),
        None,
        None,
    ) {
        Ok(graph) => graph,
        Err(e) => {
            errors.push(CompositionError::InternalError {
                message: format!("Failed to build federated query graph: {}", e),
            });
            return if errors.is_empty() { Ok(()) } else { Err(errors) };
        }
    };

    // Check field existence using the correct method name
    for (type_name, type_def) in &schema.types {
        if let ExtendedType::Object(obj) = type_def {
            for (field_name, _) in &obj.fields {
                if !obj.fields.contains_key(field_name) {
                    errors.push(CompositionError::SatisfiabilityError {
                        message: format!(
                            "Field '{}.{}' cannot be resolved across subgraphs",
                            type_name, field_name
                        ),
                    });
                }
            }
        }
    }


    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}