//! # dominant-colour
//! `dominant-colour` Command Line Interface integration tests

/// Command Line Interface integration tests
#[test]
fn cli_tests() {
    trycmd::TestCases::new().case("tests/cmd/*.toml");
}
