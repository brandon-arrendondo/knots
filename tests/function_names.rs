//! Function-name resolution must remain safe for deeply nested C declarators.

#[test]
fn deep_pointer_names_are_resolved_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let source = format!("int {}deep(void) {{ return 0; }}", "*".repeat(2_000));
            let mut parser = knots::tree_sitter::Parser::new();
            parser
                .set_language(&knots::tree_sitter_c::LANGUAGE.into())
                .unwrap();
            let tree = parser.parse(&source, None).unwrap();
            assert!(!tree.root_node().has_error());
            let function = tree.root_node().named_child(0).unwrap();
            assert_eq!(function.kind(), "function_definition");
            assert_eq!(
                knots::get_function_name(function, &source).as_deref(),
                Some("deep")
            );
            let names = knots::collect_local_names(tree.root_node(), &source);
            assert_eq!(names.len(), 1);
            assert!(names.contains("deep"));
        })
        .unwrap()
        .join()
        .unwrap();
}
