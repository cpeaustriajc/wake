//! Wake core library. No feature code yet; this crate exists so the checks
//! have something to run against.

/// The name Wake presents itself with.
#[must_use]
pub fn name() -> &'static str {
    "Wake"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_wake() {
        assert_eq!(name(), "Wake", "the product name changed");
    }
}
