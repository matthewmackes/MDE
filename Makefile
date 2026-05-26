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

.PHONY: sdist rpm test test-nodeps test-coverage smoke lint lint-grid verify rust rust-check docs iso clean install-deps install-hooks deploy deploy-rebuild deploy-status pre-cut-check

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
	@# Reject any RPM that carries `rpmlib(ShortCircuited)` — that dep
	@# is stamped on when rpmbuild runs with --short-circuit, which is
	@# a build-stage testing flag only; the resulting package refuses
	@# to install with `rpmlib(ShortCircuited) <= 4.9.0-1 is needed`.
	@# Caught one in v2.0.1's rpmbuild/RPMS/x86_64/ on 2026-05-21.
	@bad=$$(find rpmbuild/RPMS -name '*.rpm' -exec sh -c \
		'rpm -qpR "$$1" 2>/dev/null | grep -q "rpmlib(ShortCircuited)" && echo "$$1"' \
		_ {} \;); \
	if [ -n "$$bad" ]; then \
		echo "ERROR: short-circuit-tainted RPMs detected — not installable:" >&2; \
		printf '  %s\n' $$bad >&2; \
		echo "Fix: rm -rf rpmbuild/{BUILD,BUILDROOT,RPMS,SRPMS} && make rpm" >&2; \
		exit 1; \
	fi

test:
	python3 -m pytest tests/ -v

test-nodeps:
	python3 tests/_run_without_pytest.py

# v4.0.1 (2026-05-23) — EPIC-production-ready-mackes Track 4
# coverage gate. Runs pytest with coverage limited to the four
# mesh-critical modules + fails if coverage drops below 60%
# per the epic lock. Used by the release workflow as a hard
# gate before `cut release` proceeds.
#
# Coverage is intentionally NOT enforced on the whole `mackes/`
# tree — the legacy GTK panels under `mackes/workbench/*` are
# being retired in favor of `mde-workbench` and accumulating
# coverage debt on them would distort the signal.
test-coverage:
	python3 -m pytest tests/ \
		--cov=mackes.mesh_vpn \
		--cov=mackes.mesh_discovery \
		--cov=mackes.mesh_mdns \
		--cov=mackes.birthright \
		--cov-fail-under=60 \
		--cov-report=term-missing

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

# TUNE-7 (2026-05-26 per Q11 + Q12 of 25-Q tuning survey) — §0.17
# NO INCOMPLETE RELEASES enforcement. Refuses if any §11 roadmap
# epic prefix has open tasks in the worklist's Active section.
# Hard block — no operator override flag. Invoked by §0.6
# cut-release shorthand step 0 (the `cut release X.Y.Z` flow
# refuses to proceed past this gate).
pre-cut-check:
	@install-helpers/pre-cut-check.sh

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
	@if [ -e .git/hooks/commit-msg ] && [ ! -L .git/hooks/commit-msg ]; then \
		echo "✗ .git/hooks/commit-msg already exists and is not a symlink."; \
		echo "  Move it aside, then re-run \`make install-hooks\`."; \
		exit 1; \
	fi
	@ln -sfn "$$(pwd)/.claude/hooks/pre-commit-worklist.sh" .git/hooks/pre-commit
	@chmod +x .claude/hooks/pre-commit-worklist.sh
	@ln -sfn "$$(pwd)/.claude/hooks/commit-msg.sh" .git/hooks/commit-msg
	@chmod +x .claude/hooks/commit-msg.sh
	@echo "✓ .git/hooks/pre-commit  → .claude/hooks/pre-commit-worklist.sh"
	@echo "✓ .git/hooks/commit-msg  → .claude/hooks/commit-msg.sh (TUNE-9 visual-citation gate #11)"

clean:
	rm -rf build dist rpmbuild target *.egg-info
	find . -name __pycache__ -type d -exec rm -rf {} +
	find . -name "*.pyc" -delete

# v4.0.1 parity-infra entry points. `make deploy` is the "just push
# my repo into the running RPM" button — it idempotently refreshes
# the overlay script + sudoers + systemd-user units, then runs the
# overlay once. One sudo prompt, everything happens. After the first
# `make deploy`, every `git commit` on main auto-deploys via the
# systemd-user path-watch — no manual rebuilds.
#
# `make deploy-rebuild` reruns the overlay on demand (e.g. for an
# uncommitted edit). Falls back to a full `make deploy` if the
# overlay script isn't installed yet.
#
# `make deploy-status` shows whether the watch is alive + the last
# log lines.
deploy:
	@echo "==> sudo install-helpers/install-parity-infra.sh"
	@sudo install-helpers/install-parity-infra.sh

deploy-rebuild:
	@if [ -x /usr/local/bin/mde-parity-overlay ]; then \
		/usr/local/bin/mde-parity-overlay; \
	else \
		echo "==> overlay not installed yet — running 'make deploy' first"; \
		sudo install-helpers/install-parity-infra.sh; \
	fi

deploy-status:
	@systemctl --user status mde-parity.path --no-pager 2>&1 | head -10
	@echo "----------------------------------------"
	@echo "==> last 20 log lines (sudo may prompt)"
	@sudo tail -20 /var/log/mde-parity.log 2>/dev/null \
		|| echo "(no log yet — overlay never ran)"
