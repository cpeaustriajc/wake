default: check

# One recipe per check. To add a check, write its recipe and name it here.
check: format unused-dependencies dependencies lint test

format:
    cargo fmt --check

lint:
    cargo clippy --all-targets --locked -- -D warnings

dependencies:
    cargo deny check

unused-dependencies:
    cargo machete

test:
    cargo test --locked

# The GPUI window app (apps/wake-window). Needs macOS (ADR-014); CI runs it on a macOS runner.
window:
    cargo clippy -p wake-window --all-targets --locked -- -D warnings
    cargo build -p wake-window --locked

fix:
    cargo fmt
    cargo clippy --all-targets --locked --fix --allow-dirty --allow-staged
