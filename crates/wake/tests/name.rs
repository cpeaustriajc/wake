//! Tests for the product name.

use wake::name;

#[test]
fn name_is_wake() {
    assert_eq!(name(), "Wake", "the product name changed");
}
