.PHONY: check ci check-commits fmt clippy test

# Local pre-push (includes commit subjects).
check: fmt clippy test check-commits

# CI and quick verify (commit subjects validated in conventional-commits.yml).
ci: fmt clippy test

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
