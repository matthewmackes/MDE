# Mackes Shell — build helpers.
#
# `make sdist`      → dist/mackes-shell-<version>.tar.gz
# `make rpm`        → built RPM under rpmbuild/RPMS/noarch/
# `make test`       → pytest (requires pytest installed)
# `make smoke`      → import-graph smoke check (no pytest needed)
# `make rust`       → cargo build --release (Phase 0.1+ Rust components)
# `make rust-check` → cargo check + clippy + rustfmt --check (CI gate)
# `make clean`      → remove build artifacts

NAME    := mackes-shell
VERSION := $(shell python3 -c "import mackes; print(mackes.__version__)")
SDIST   := dist/$(NAME)-$(VERSION).tar.gz

.PHONY: sdist rpm test test-nodeps smoke lint lint-grid verify rust rust-check docs iso clean install-deps install-hooks

sdist:
	@# Prefer PEP 517 build (works on Fedora 40+ without distutils).
	@if python3 -c "import build" 2>/dev/null; then \
		python3 -m build --sdist; \
	else \
		python3 setup.py sdist; \
	fi
	@# PEP 625 normalizes the name to `mackes_shell` in the sdist filename;
	@# the .spec's Source0 expects the hyphenated form. Provide both —
	@# always overwrite the hyphen alias so a stale prior-session copy
	@# doesn't shadow a freshly-built underscore tarball.
	@if [ -f dist/mackes_shell-$(VERSION).tar.gz ]; then \
		cp -f dist/mackes_shell-$(VERSION).tar.gz $(SDIST); \
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

# Mirrors .github/workflows/ci.yml's ruff gate exactly so a local
# pass means ci will pass too. Pre-commit gate — see
# .claude/CLAUDE.md §0.7.
lint:
	ruff check --select F401,F541,F811,F841 mackes/ tests/

# UX-12 (2026-05-21) — modular spacing-grid lint for Iced sources.
# Currently warn-only; will flip to strict once UX-2..UX-9 land
# their consumer-side migration to mde-theme tokens.
lint-grid:
	@tools/mde-grid-lint.sh

smoke:
	python3 -c "import importlib, pkgutil, sys, mackes; \
fails=[]; \
[ (importlib.import_module(n) ) for _,n,_ in pkgutil.walk_packages(mackes.__path__, prefix='mackes.') ]; \
print('smoke OK')"

# WF-2 (2026-05-21) — aggregate pre-commit gate. Mirrors §0.7 of
# .claude/CLAUDE.md. Runs only the gates relevant to staged changes,
# detected via `git diff --name-only`. ci.yml calls this same target
# so local-pass means ci-pass.
verify:
	@set -e; \
	CHANGED="$$(git diff --cached --name-only 2>/dev/null || true)"; \
	if [ -z "$$CHANGED" ]; then \
		CHANGED="$$(git diff --name-only HEAD 2>/dev/null || true)"; \
	fi; \
	echo "verify: scanning changed files…"; \
	echo "$$CHANGED"; \
	NEED_PY=0; NEED_RUST=0; NEED_CSS=0; NEED_PKG=0; \
	for f in $$CHANGED; do \
		case "$$f" in \
			mackes/*|tests/*) NEED_PY=1 ;; \
			crates/*) NEED_RUST=1 ;; \
			data/css/*) NEED_CSS=1 ;; \
			packaging/*|setup.py|pyproject.toml|data/*|mackes/birthright.py) NEED_PKG=1 ;; \
		esac; \
	done; \
	if [ $$NEED_PY -eq 1 ]; then \
		echo "→ python: smoke + test-nodeps + lint"; \
		$(MAKE) smoke; \
		$(MAKE) test-nodeps; \
		$(MAKE) lint; \
	fi; \
	if [ $$NEED_RUST -eq 1 ]; then \
		echo "→ rust: rust-check (fmt + clippy + check)"; \
		$(MAKE) rust-check; \
		echo "→ rust: grid lint (warn-only)"; \
		$(MAKE) lint-grid; \
	fi; \
	if [ $$NEED_CSS -eq 1 ] && [ -x install-helpers/lint-css.sh ]; then \
		echo "→ css: install-helpers/lint-css.sh"; \
		install-helpers/lint-css.sh; \
	fi; \
	if [ $$NEED_PKG -eq 1 ]; then \
		echo "→ packaging touched — recommend running \`make rpm\` before commit"; \
	fi; \
	echo "verify: ok"

iso:
	@command -v livemedia-creator >/dev/null 2>&1 \
	  || { echo 'Install lorax: sudo dnf install lorax pykickstart' >&2; exit 1; }
	mkdir -p dist/iso
	# CB-4.4 — v2.0.0 ISO builds from packaging/iso/mde.ks (the
	# v1.x mackes-xfce.ks was deleted at CB-4.1). Volid + project
	# names flip to MDE.
	sudo livemedia-creator \
	    --make-iso \
	    --ks packaging/iso/mde.ks \
	    --no-virt \
	    --resultdir dist/iso \
	    --project "Mackes Desktop Environment" \
	    --releasever "$$(rpm -E %fedora)" \
	    --volid "MDE"

rust:
	cargo build --release --workspace

rust-check:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo check --workspace --all-targets

# Phase 12.12.2 — library reference. Renders the public API for
# `mackesd-core` (and the rest of the workspace's libs) as HTML +
# stashes it where the Workbench Help tab can link to it.
docs:
	cargo doc --no-deps --workspace
	@echo "Generated docs under target/doc/. Install target:"
	@echo "  sudo cp -r target/doc /usr/share/mackes-shell/help/cargo-doc/"

install-deps:
	@echo 'On Fedora: sudo dnf install python3-pytest python3-pyyaml python3-gobject gtk3 xfconf xfce4-whiskermenu-plugin xfce4-pulseaudio-plugin xfce4-power-manager-plugin rust cargo rustfmt clippy'

# WF-5.a (2026-05-21) — install the git pre-commit hook from
# .claude/hooks/. Idempotent; never touches git config.
install-hooks:
	@if [ ! -d .git ]; then \
		echo "✗ .git/ not found — run from repo root"; exit 1; \
	fi
	@if [ -e .git/hooks/pre-commit ] && [ ! -L .git/hooks/pre-commit ]; then \
		echo "✗ .git/hooks/pre-commit already exists and is not a symlink."; \
		echo "  Move it aside, then re-run \`make install-hooks\`."; \
		exit 1; \
	fi
	@ln -sfn "$$(pwd)/.claude/hooks/pre-commit-worklist.sh" .git/hooks/pre-commit
	@chmod +x .claude/hooks/pre-commit-worklist.sh
	@echo "✓ .git/hooks/pre-commit → .claude/hooks/pre-commit-worklist.sh"

clean:
	rm -rf build dist rpmbuild target *.egg-info
	find . -name __pycache__ -type d -exec rm -rf {} +
	find . -name "*.pyc" -delete
