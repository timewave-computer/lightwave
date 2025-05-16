# Default slot number if not provided
SLOT ?= 11715392

.PHONY: build-circuits

build-circuits:
	cargo run --bin service --release -- --delete
	cargo run --bin service --release -- --generate-recursion-circuit $(SLOT)
	cargo run --bin service --release -- --dump-elfs
	cargo run --bin service --release -- --generate-wrapper-circuit
	cargo run --bin service --release -- --dump-elfs 

run:
	cargo run --bin service --release -- --delete
	cargo run --bin service --release 

continue:
	cargo run --bin service --release 

