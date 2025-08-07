# Apollo Federation Composition Implementation


## Overview
- This document outlines the implementation of core Apollo Federation composition logic in Rust, mirroring the functionality of the Node.js reference (compose.ts). The primary goal is to validate, merge, and finalize a supergraph schema from multiple subgraphs.

## Objective
- Port the core composition logic from the Node.js implementation (compose.ts) to the Rust implementation
inside apollo-federation/src/composition/mod.rs. The goal was to bring the compose function in Rust to feature parity with the JS version by implementing the following:

- pre_merge_validations(...)
- merge_subgraphs(...)
- post_merge_validations(...)

## What Was Done
1. pre_merge_validations(...)
- Purpose:
    - Ensures all subgraphs are valid before attempting to merge. Detects duplicate schemas and validates each subgraph’s federation compliance.schema.
    - Key Logic:
        - Uses schema_string() to detect duplicate schemas.
        - Attempts to construct ValidFederationSchema from each subgraph.
        - Collects ```CompositionError::TypeDefinitionInvalid``` or SubgraphError.

```
pub fn pre_merge_validations(
    subgraphs: &[Subgraph<Validated>],
) -> Result<(), Vec<CompositionError>>

```

2. merge_subgraphs(...)
- Purpose:
    - Takes validated subgraphs and produces a merged Supergraph schema.
     
    - Key Logic:
        - Converts each subgraph to a ValidFederationSubgraph.
        - Populates a ValidFederationSubgraphs struct.
        - Uses merge_federation_subgraphs(...) to merge.
        - Returns a Supergraph<Merged> on success or a list of merge errors.

```
pub fn merge_subgraphs(
    subgraphs: Vec<Subgraph<Validated>>,
) -> Result<Supergraph<Merged>, Vec<CompositionError>>

```

3. post_merge_validations(...)
- Purpose:
    - Ensures the resulting supergraph is valid and executable.
    - Key Logic:
      - Confirms schema is non-empty.
      - Ensures presence of a Query root type.
      - Validates schema structure.
      - Attempts to build a federated query graph.
      - Checks for unresolved fields.
```
pub fn post_merge_validations(
    supergraph: &Supergraph<Merged>,
) -> Result<(), Vec<CompositionError>>

```
## Tests & Integration
- I wrote extensive unit tests to verify each composition step. These tests were placed in a standalone integration test file: apollo-federation/tests/assessment_test.rs.

- Test Cases Implemented
   - test_pre_merge_validations_passes_with_unique_subgraphs
       - Verifies duplicate subgraph detection.
   - test_merge_subgraphs_success
       - Ensures a valid subgraph merges successfully.
   - test_post_merge_validations_success
       - Validates a proper supergraph passes post-merge checks.
   - test_post_merge_validations_fails_on_missing_query
       - Ensures error is raised for missing root Query type.
       - 
### Supporting Test Helpers for test functions
- create_dummy_validated_subgraph()
  - Parses and validates a small schema, used across tests.
- create_dummy_supergraph()
    - Constructs a minimal valid Supergraph<Merged>. 
    
