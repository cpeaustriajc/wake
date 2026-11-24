default: check

check: format unused-dependencies dependencies lint test-location test

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

test-location:
    #!/bin/sh
    found=$(grep -rlE '#\[(test|cfg\(test\))\]' --include='*.rs' apps/*/src crates/*/src 2>/dev/null | tr '\n' ' ')
    [ -z "$found" ] && exit 0
    echo "tests belong under tests/, not src/: $found" >&2
    exit 1

window:
    cargo clippy -p wake-window --all-targets --locked -- -D warnings
    cargo build -p wake-window --locked

hooks:
    git config core.hooksPath .githooks

fix:
    cargo fmt
    cargo clippy --all-targets --locked --fix --allow-dirty --allow-staged

resume-test *flags:
    cargo build --locked -p wake-cli
    cargo run --locked -p wake-resume-test -- results target/debug/wake {{flags}}

record-fixtures:
    cargo build --locked -p wake-cli
    cargo run --locked -p wake-record-fixtures -- crates/wake-claude/tests/fixtures target/debug/wake
