# Mackes Shell — build helpers.
#
# `make sdist`      → dist/mackes-shell-<version>.tar.gz
# `make rpm`        → built RPM under rpmbuild/RPMS/noarch/
# `make test`       → pytest (requires pytest installed)
# `make smoke`      → import-graph smoke check (no pytest needed)
# `make clean`      → remove build artifacts

NAME    := mackes-shell
VERSION := $(shell python3 -c "import mackes; print(mackes.__version__)")
SDIST   := dist/$(NAME)-$(VERSION).tar.gz

.PHONY: sdist rpm test smoke clean install-deps

sdist:
	@# Prefer PEP 517 build (works on Fedora 40+ without distutils).
	@if python3 -c "import build" 2>/dev/null; then \
		python3 -m build --sdist; \
	else \
		python3 setup.py sdist; \
	fi
	@# PEP 503 normalizes the name to `mackes_shell` in the sdist filename;
	@# the .spec's Source0 expects the hyphenated form. Provide both.
	@if [ -f dist/mackes_shell-$(VERSION).tar.gz ] && [ ! -f $(SDIST) ]; then \
		cp dist/mackes_shell-$(VERSION).tar.gz $(SDIST); \
	fi
	@ls -la dist/

rpm: sdist
	mkdir -p rpmbuild/{SOURCES,SPECS,BUILD,RPMS,SRPMS}
	cp $(SDIST) rpmbuild/SOURCES/
	cp packaging/fedora/$(NAME).spec rpmbuild/SPECS/
	rpmbuild --define '_topdir $(CURDIR)/rpmbuild' \
		-ba rpmbuild/SPECS/$(NAME).spec

test:
	python3 -m pytest tests/ -v

test-nodeps:
	python3 tests/_run_without_pytest.py

smoke:
	python3 -c "import importlib, pkgutil, sys, mackes; \
fails=[]; \
[ (importlib.import_module(n) ) for _,n,_ in pkgutil.walk_packages(mackes.__path__, prefix='mackes.') ]; \
print('smoke OK')"

iso:
	@command -v livemedia-creator >/dev/null 2>&1 \
	  || { echo 'Install lorax: sudo dnf install lorax pykickstart' >&2; exit 1; }
	mkdir -p dist/iso
	sudo livemedia-creator \
	    --make-iso \
	    --ks packaging/iso/mackes-xfce.ks \
	    --no-virt \
	    --resultdir dist/iso \
	    --project "Mackes XFCE" --releasever "$$(rpm -E %fedora)" \
	    --volid "MACKES_XFCE"

install-deps:
	@echo 'On Fedora: sudo dnf install python3-pytest python3-pyyaml python3-gobject gtk3 xfconf xfce4-whiskermenu-plugin xfce4-pulseaudio-plugin xfce4-power-manager-plugin'

clean:
	rm -rf build dist rpmbuild *.egg-info
	find . -name __pycache__ -type d -exec rm -rf {} +
	find . -name "*.pyc" -delete
