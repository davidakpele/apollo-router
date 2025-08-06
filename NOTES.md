# Apollo Federation Composition Implementation


## Overview
This document details the Rust implementation of Apollo Federation composition functions ported from the Node.js reference implementation. The core functionality validates and merges GraphQL subgraphs into a federated supergraph schema.

## Objective
- Port the core composition logic from the Node.js implementation (compose.ts) to the Rust implementation
inside apollo-federation/src/composition/mod.rs. The goal was to bring the compose function in Rust to feature parity with the JS version by implementing the following:

- pre_merge_validations(...)
- merge_subgraphs(...)
- post_merge_validations(...)

## What Was Done
1. pre_merge_validations(...)
- Purpose:
    - This function performs initial validations on the subgraphs before attempting a merge. It ensures no duplicate schemas exist and validates each subgraph’s schema.
    
    - Key Logic:
        -Tracks subgraphs via their schema_string() and detects duplicates.
        - Attempts to create a ValidFederationSchema for each subgraph.
        - Collects and returns any encountered CompositionError.

```
pub fn pre_merge_validations(
    subgraphs: &[Subgraph<Validated>],
) -> Result<(), Vec<CompositionError>> {
    ...
}
```

2. merge_subgraphs(...)
- Purpose:
    - This function converts validated subgraphs into a ValidFederationSubgraphs structure and uses merge_federation_subgraphs to produce a merged supergraph.
     
    - Key Logic:
        - Converts each subgraph schema into a ValidFederationSchema.
        - Collects them into a BTreeMap used by ValidFederationSubgraphs.
        - Uses the merger function merge_federation_subgraphs.
        - Returns a Supergraph<Merged> on success or collects merge errors. 

```
pub fn merge_subgraphs(
    subgraphs: Vec<Subgraph<Validated>>,
) -> Result<Supergraph<Merged>, Vec<CompositionError>> {
    ...
}
```

3. post_merge_validations(...)
- Purpose:
    - This step ensures that the merged supergraph schema is structurally valid post-merge.

- Status:
    - Implemented or left for final implementation and testing (based on context — include implementation details if already written).