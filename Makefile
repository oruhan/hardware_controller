PREFIX ?= /usr/local
UDEV_DIR ?= /usr/lib/udev/rules.d
DESTDIR ?=

.PHONY: build check install uninstall

build:
	cargo build --release --locked --bin hardware-controller --bin razerctl

check:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets --locked -- -D warnings
	cargo test --workspace --all-targets --locked

install: build
	install -Dm755 target/release/hardware-controller "$(DESTDIR)$(PREFIX)/bin/hardware-controller"
	install -Dm755 target/release/razerctl "$(DESTDIR)$(PREFIX)/bin/razerctl"
	install -Dm644 packaging/hardware-controller.desktop "$(DESTDIR)$(PREFIX)/share/applications/hardware-controller.desktop"
	install -Dm644 packaging/io.github.oruhan.hardware_controller.metainfo.xml "$(DESTDIR)$(PREFIX)/share/metainfo/io.github.oruhan.hardware_controller.metainfo.xml"
	install -Dm644 crates/gui/assets/mouse.svg "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/hardware-controller.svg"
	install -Dm644 packaging/udev/70-hardware-controller-razer.rules "$(DESTDIR)$(UDEV_DIR)/70-hardware-controller-razer.rules"

uninstall:
	rm -f "$(DESTDIR)$(PREFIX)/bin/hardware-controller"
	rm -f "$(DESTDIR)$(PREFIX)/bin/razerctl"
	rm -f "$(DESTDIR)$(PREFIX)/share/applications/hardware-controller.desktop"
	rm -f "$(DESTDIR)$(PREFIX)/share/metainfo/io.github.oruhan.hardware_controller.metainfo.xml"
	rm -f "$(DESTDIR)$(PREFIX)/share/icons/hicolor/scalable/apps/hardware-controller.svg"
	rm -f "$(DESTDIR)$(UDEV_DIR)/70-hardware-controller-razer.rules"
