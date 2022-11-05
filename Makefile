.PHONY: test format

test:
	cargo fmt -- --check
	cargo-sort --check --workspace
	cargo clippy --all-features --workspace -- -D warnings
	cargo test --all-features --workspace
	cargo test --no-default-features --workspace

format:
	cargo fmt
	cargo-sort --workspace
