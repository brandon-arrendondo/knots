// probe: comment markers in Rust: nested block comments and markers inside strings
// source: Park 1992 (physical lines holding code); the Rust Reference, "Comments": block comments nest, so
//         `/* a /* b */ c */` is one comment, and `"/*"` is a string. f: the signature, the two let lines, the
//         return line and the closing brace = 5.
// expect f sloc=5
fn f(x: i32) -> i32 {
    let open = "/*";
    /* outer /* inner */
       still the outer comment */
    let y = x + open.len() as i32;
    y
}
