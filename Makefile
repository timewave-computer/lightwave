.PHONY: build-circuits

build-circuits:
	cargo run --bin service --release -- --delete
	cargo run --bin service --release -- --generate-recursion-circuit
	cargo run --bin service --release -- --dump-elfs
	cargo run --bin service --release -- --generate-wrapper-circuit
	cargo run --bin service --release -- --dump-elfs 

run:
	cargo run --bin service --release -- --delete
	cargo run --bin service --release 

continue:
	cargo run --bin service --release 

