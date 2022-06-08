bin:
	cargo build --release

install:
	cp target/release/isulad-agent /usr/local/bin/

.PHONY: bin install