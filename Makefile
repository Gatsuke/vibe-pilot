.PHONY: build test lint fmt bench clean

build:
	cargo build --release

test:
	cargo test

lint:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

bench:
	bash bench/run.sh

clean:
	cargo clean
	rm -rf /tmp/vp-bench-app
