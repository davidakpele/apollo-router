

    use apollo_compiler::Schema;
    use apollo_compiler::validation::Valid;
    use apollo_federation::{composition::{merge_subgraphs, post_merge_validations, pre_merge_validations}, error::CompositionError, subgraph::typestate::{Subgraph, Validated}, supergraph::{Merged, Supergraph}};

    fn create_dummy_validated_subgraph() -> Subgraph<Validated> {
        use apollo_compiler::Schema;
        use apollo_federation::{
            subgraph::typestate::{Initial, Subgraph},
            composition::upgrade_subgraphs_if_necessary, // ✅ Use your re-exported version here
        };

        let raw_sdl = r#"
            type Query {
                hello: String
            }
        "#;

        // Step 1: Parse schema
        let parsed_schema = Schema::parse(raw_sdl, "test.graphql").expect("schema parse failed");

        // Step 2: Create initial subgraph
        let subgraph = Subgraph::<Initial>::new("TestSubgraph", "http://localhost", parsed_schema);

        // Step 3: Expand links
        let expanded = subgraph.expand_links().expect("expand_links failed");

        // Step 4: Upgrade using your public helper
        let upgraded_vec = upgrade_subgraphs_if_necessary(vec![expanded]).expect("upgrade failed");
        let upgraded = upgraded_vec.into_iter().next().expect("no subgraph");

        // Step 5: Validate
        let validated = upgraded.validate().expect("validation failed");

        validated
    }


    #[allow(dead_code)]
    fn create_dummy_supergraph() -> Supergraph<Merged> {
        let schema_sdl = r#"
            schema {
                query: Query
            }

            type Query {
                hello: String
            }
        "#;

        let schema = Schema::parse(schema_sdl, "supergraph.graphql").expect("schema parse failed");
        let valid_schema = Valid::assume_valid(schema);

        Supergraph::<Merged>::new(valid_schema)
    }

    #[test]
    fn test_pre_merge_validations_passes_with_unique_subgraphs() {
        let subgraph1 = create_dummy_validated_subgraph();
        let subgraph2 = create_dummy_validated_subgraph(); // different instance, same content
        let result = pre_merge_validations(&[subgraph1, subgraph2]);

        // Since schema strings are the same, this should error
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, CompositionError::TypeDefinitionInvalid { .. })));
    }

    #[test]
    fn test_merge_subgraphs_success() {
        let subgraph = create_dummy_validated_subgraph();
        let result = merge_subgraphs(vec![subgraph]);
        assert!(result.is_ok());

        let supergraph = result.unwrap();
        assert!(!supergraph.schema().types.is_empty());
    }

    #[test]
    fn test_post_merge_validations_success() {
        let validated = create_dummy_validated_subgraph();
        let supergraph = merge_subgraphs(vec![validated]).expect("merge failed");
        let result = post_merge_validations(&supergraph);
        assert!(result.is_ok());
    }

    #[test]
    fn test_post_merge_validations_fails_on_missing_query() {
        let schema_sdl = r#"
            type Mutation {
                doSomething: String
            }
        "#;

        let schema = Schema::parse(schema_sdl, "supergraph.graphql").expect("schema parse failed");
        let valid_schema = Valid::assume_valid(schema);
        let supergraph = Supergraph::<Merged>::new(valid_schema);

        let result = post_merge_validations(&supergraph);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| match e {
            CompositionError::TypeDefinitionInvalid { message } => {
                message.contains("Missing root Query type")
            }
            _ => false,
        }));
    }

