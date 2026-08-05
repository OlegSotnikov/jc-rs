.DEFAULT_GOAL := help
SHELL := /bin/bash

BIN := target/release/jc-rs
JC  := jc

.PHONY: help
help: ## show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
	  | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.PHONY: build
build: ## release build
	cargo build --release

.PHONY: test
test: ## cargo tests (unit + integration); red by design, see ci/known-failures.txt
	TZ=PST8PDT cargo test --workspace

.PHONY: ratchet
ratchet: ## the real test gate: fails on new failures AND on known ones that pass
	./ci/run-tests.sh

.PHONY: sync-fixtures
sync-fixtures: submodule ## overwrite tests/fixtures from the pinned jc source
	@cp -a $(JC)/tests/fixtures/. tests/fixtures/
	@echo "tests/fixtures now mirrors jc $$(cd $(JC) && git describe --tags)"
	@echo "note: fixtures jc does not ship (e.g. bluetoothctl_*) are left alone"

.PHONY: check-fixtures
check-fixtures: submodule ## fail if any fixture we share with jc has been edited
	@# Only files that exist on both sides are compared. A handful of fixtures
	@# are this project's own test data and have no jc counterpart; those are
	@# fine. What must never happen is a fixture jc DOES ship being edited here
	@# to match our output; that is how a 100% compatibility claim gets made
	@# without being true.
	@drift=$$(diff -rq tests/fixtures $(JC)/tests/fixtures 2>/dev/null | grep '^Files ' || true); \
	if [ -n "$$drift" ]; then \
	  echo "$$drift"; \
	  echo "a fixture shared with jc has been modified; run: make sync-fixtures"; \
	  exit 1; \
	fi; \
	echo "fixtures in sync with jc"

.PHONY: lint
lint: ## clippy + fmt check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo fmt --all -- --check

.PHONY: submodule
submodule: ## check out the pinned jc source (the schema authority)
	git submodule update --init
	@echo "jc pinned at: $$(cd $(JC) && git describe --tags 2>/dev/null || git rev-parse --short HEAD)"

.PHONY: deps-py
deps-py: ## python deps the differential oracle needs
	python3 -m pip install --user xmltodict ruamel.yaml pygments

.PHONY: differential
differential: build submodule ## run jc-rs against the FULL jc fixture corpus
	python3 tests/differential/validate.py

.PHONY: differential-gate
differential-gate: build submodule ## the release gate: 100% of oracle-valid pairs
	python3 tests/differential/validate.py --fail-under 100

.PHONY: bench
bench: ## criterion benchmarks
	cargo bench -p jc-rs-bench

.PHONY: homebrew-formula
homebrew-formula: ## regenerate Formula/jc-rs.rb for TAG (default: latest release)
	@./ci/homebrew-formula.sh

.PHONY: wasm
wasm: ## build the npm package into crates/jc-rs-wasm/pkg (needs wasm-pack)
	@command -v wasm-pack >/dev/null || { \
	  echo "wasm-pack not installed: cargo install wasm-pack"; exit 1; }
	wasm-pack build crates/jc-rs-wasm --release --target web --out-name jc-rs
	node crates/jc-rs-wasm/tests/smoke.mjs
	@echo "package at crates/jc-rs-wasm/pkg"

.PHONY: bench-vs-jc
bench-vs-jc: build submodule ## time jc-rs against jc on the same inputs
	./ci/bench-vs-jc.sh

.PHONY: check
check: lint check-fixtures ratchet differential ## universal verification: lint + fixture sync + test ratchet + differential
	@echo "check complete"

.PHONY: release
release: ## cut a release: make release VERSION=0.2.0 (bumps, tags, pushes; publishing follows)
	@test -n "$(VERSION)" || { echo "usage: make release VERSION=0.2.0"; exit 1; }
	@./ci/release.sh $(VERSION)

.PHONY: clean
clean:
	cargo clean
