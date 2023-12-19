SHELL = /bin/sh

bindir := bin
INSTALLDIR := $(DESTDIR)$(bindir)

.PHONY: all check install clean distclean

all:
	cargo build --release

check:
	cargo test --release

install: all check
	mkdir -p $(INSTALLDIR)
	cp target/release/decode-metar $(INSTALLDIR)/.

clean:
	rm -rf target

distclean: clean
	rm -f Cargo.lock
