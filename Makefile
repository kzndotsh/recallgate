.PHONY: check check-commits fmt clippy test

check: fmt clippy test check-commits

check-commits:
	@chmod +x scripts/validate-conventional-commits.sh
	@base="$$(git merge-base HEAD origin/main 2>/dev/null || echo HEAD~1)"; \
	./scripts/validate-conventional-commits.sh "$${base}..HEAD"

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace
