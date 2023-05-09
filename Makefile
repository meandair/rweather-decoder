check:
	cargo check

debug:
	cargo build

release:
	cargo build --release

test:
	cargo test --release

docs:
	cargo doc

install: release test docs
	mkdir install
	cp target/release/decode-metar install/.

clean:
	cargo clean
	rm -f Cargo.lock
