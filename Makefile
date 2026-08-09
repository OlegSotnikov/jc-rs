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
test: ## cargo tests (unit + integration)
	TZ=PST8PDT cargo test --workspace

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
bench-vs-jc: submodule ## time jc-rs against jc on the same inputs
	@# The published numbers time the published binary. On Linux that is the
	@# static musl build, which carries mimalloc and is therefore not the same
	@# program as `make build` produces; timing the convenient one and printing
	@# it next to a download link would be a quiet lie.
	@if rustup target list --installed 2>/dev/null | grep -q x86_64-unknown-linux-musl; then \
	  CC_x86_64_unknown_linux_musl=$${CC_x86_64_unknown_linux_musl:-musl-gcc} \
	    cargo build --release --target x86_64-unknown-linux-musl -p jc-rs && \
	  BIN=target/x86_64-unknown-linux-musl/release/jc-rs ./ci/bench-vs-jc.sh; \
	else \
	  echo "musl target absent; timing the local glibc build instead."; \
	  echo "for the published numbers: rustup target add x86_64-unknown-linux-musl"; \
	  echo "                           apt install musl-tools   # mimalloc needs a C compiler"; \
	  cargo build --release -p jc-rs && ./ci/bench-vs-jc.sh; \
	fi

.PHONY: site-wasm
site-wasm: ## build the wasm bundle the website's converter runs on
	wasm-pack build crates/jc-rs-wasm --release --target web --out-name jc-rs
	@mkdir -p website/public/wasm
	@cp crates/jc-rs-wasm/pkg/jc-rs.js crates/jc-rs-wasm/pkg/jc-rs_bg.wasm website/public/wasm/
	@echo "website/public/wasm updated ($$(du -h website/public/wasm/jc-rs_bg.wasm | cut -f1))"

.PHONY: site-data
site-data: build ## regenerate the website's parser dataset from this repo
	TZ=PST8PDT python3 website/scripts/build-data.py

.PHONY: site
site: site-data site-wasm ## everything the website build needs from this repo
	@echo "now: cd website && npm ci && npm run build"

.PHONY: check
check: lint check-fixtures test differential ## universal verification: lint + fixture sync + tests + differential
	@echo "check complete"

.PHONY: release
release: ## cut a release: make release VERSION=0.2.0 (bumps, tags, pushes; publishing follows)
	@test -n "$(VERSION)" || { echo "usage: make release VERSION=0.2.0"; exit 1; }
	@./ci/release.sh $(VERSION)

.PHONY: clean
clean:
	cargo clean
