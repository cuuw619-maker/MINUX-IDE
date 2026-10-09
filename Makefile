.PHONY: build test check themes audit clean dev

build:
	cargo build --release

dev:
	cargo run

themes:
	python scripts/generate_themes.py

audit:
	python scripts/project_audit.py .

test: audit
	cargo test
	npm run typecheck
	npm run test:js

check: test

clean:
	cargo clean
