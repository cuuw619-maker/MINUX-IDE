.PHONY: build test check themes audit clean dev kotlin-test

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
	gradle -p kotlin test

kotlin-test:
	gradle -p kotlin test

check: test

clean:
	cargo clean
