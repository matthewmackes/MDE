# MDE security posture

**Lock date:** 2026-05-26 (TUNE-12, 25-Q tuning survey aftermath)
**Authority:** AI_GOVERNANCE.md §7 — this doc expands the lock into
the deliberate posture statement that closes AI_PLATFORM_REFERENCE.md
§11.5 ("what replaces the in-process AppArmor / SELinux posture").

This is the platform's stated security posture. It is **not** an
incident-response runbook, threat model exercise, or compliance
mapping — those are post-1.0 concerns. The goal here is to make the
deliberate choices explicit so a security reviewer can see the
posture rather than infer one.

---

## 1. The single-sentence posture

MDE runs every component as the operator's user UID inside a Fedora
target with SELinux enforcing, exposes only two public ports
(UDP/4242 + TCP/443 on lighthouses), and uses the Nebula overlay as
the transport-encryption + authentication boundary. The mesh
interior is flat-trust by design — one passcode, every peer
equal — and the boundary is the Nebula CA + the bind-scope
enforced by the lint gate (CLAUDE.md §0.7 #10).

---

## 2. The policy base — Fedora targeted SELinux

MDE targets **Fedora** (no other distro support in 1.0). The kickstart
(`packaging/iso/mde.ks:25`) ships `selinux --enforcing`. Every
shipped systemd unit, daemon, and helper runs under the **Fedora
targeted policy** — the same policy that ships with every Fedora
Workstation install.

**No custom MDE SELinux policy module ships in 1.0.** Per-component
type labels (e.g. `mded_t`, `mackesd_t`, `nebula_mde_t`) are a
deliberate post-1.0 deferral; their absence is documented here as
the conscious choice rather than an oversight. The reasoning:

- Every MDE component already runs as the operator's user UID
  (see §3). The targeted policy's `user_t` / `unconfined_t` /
  `staff_t` types correctly bound those processes from accessing
  system files outside the user's home + the platform's
  shared-state directories.
- Authoring per-component types adds maintenance surface that pays
  off only when the components run as system UIDs (root or a
  dedicated `mded` user). MDE doesn't.
- Fedora's targeted policy already labels the standard mount
  points, capabilities, and network interfaces the platform
  touches. The added value of a custom module would be marginal
  for the 1.0 deployment profile (one operator, ≤8 peers, no
  shared OS users).

**If 1.x evolves toward** multi-user-on-one-peer or shared-OS-user
deployments, a custom policy module becomes load-bearing. That's
the trigger to author one — not an arbitrary version boundary.

---

## 3. The process model — user-UID isolation

Every long-running MDE process is **owned by the operator's user
UID**. There is no system service that runs as root after boot
besides the Nebula data-plane service itself (see §4). The
practical implications:

- `mded` runs under `systemd --user`, owned by `$XDG_RUNTIME_DIR`
  semantics, lives + dies with the operator's session.
- `mackesd` (the platform supervisor) runs under the operator's
  UID via `systemd --user` units shipped in
  `data/systemd/mackesd.service`.
- Per-crate workers (`mde-bus`, `mde-clipd`, `mde-popover`,
  `mde-workbench`, `mde-files`, `mde-kdc`, `mde-music`, etc.) are
  all spawned as the operator's UID — either as direct
  `systemd --user` units or as worker children of `mackesd`.
- Cross-peer mounts (Gluster `mesh-home`, KDC2 drop folders) live
  under `~/Documents` / `~/Pictures` / `~/Videos` / `~/Music` /
  `~/Downloads` — owned by the operator's UID, traversable only by
  that UID.

**The only system-UID process is the Nebula data plane.** Its
isolation is documented in §4. No other shipped component
requires root after boot — installation + birthright pairing use
`pkexec` for one-shot privileged actions and return immediately
(`mackes/admin_session.py` enforces this contract).

The flip side: every component shares a fate. A compromise of the
operator's session compromises every MDE process. This is
deliberate — the platform targets the single-operator-per-peer
profile (Q98 lock); session isolation between MDE components would
add no defensive value against the actual threat model (operator
loses control of their own session). See §5 for the threat-model
discussion.

---

## 4. The Nebula data plane — `CAP_NET_ADMIN` scoped via systemd

Nebula is the one piece of the platform that needs a capability the
operator's UID doesn't have: `CAP_NET_ADMIN` to create + manage the
`nebula1` (or `tun0`) virtual interface. The platform scopes this
narrowly via the systemd unit files at `data/systemd/nebula.service`
+ `data/systemd/nebula-lighthouse.service`:

```ini
[Service]
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_BIND_SERVICE
NoNewPrivileges=true
# (additional hardening: ProtectSystem, ProtectHome, etc. — see unit file)
```

The Nebula binary runs **without** `CAP_NET_RAW`, `CAP_SYS_ADMIN`,
`CAP_DAC_OVERRIDE`, or any other capability outside the two listed
above. The `CapabilityBoundingSet` line is the kernel-enforced
ceiling — even a compromised Nebula binary cannot acquire a
capability outside this set.

**The AC's "scoped via `nebula_t`" framing** anticipates a future
custom SELinux type for the Nebula process. **1.0 does not ship
one** — the systemd capability scoping is the scoping mechanism.
The trade-off:

- **What we get:** kernel-enforced capability ceiling, no SUID
  binaries, no setcap on the executable itself (the systemd unit
  is the only path to those capabilities), `NoNewPrivileges=true`
  preventing privilege escalation via execve.
- **What we don't get:** an SELinux type that would block file
  + network access outside Nebula's expected paths. The Fedora
  targeted policy's default unconfined system-service type
  (`nebula_t` or `unconfined_service_t` depending on the policy
  version) provides only weak file-system labeling.
- **The defense-in-depth gap** that authoring `nebula_t` would
  close: a compromised Nebula process could read arbitrary
  files within the targeted-policy ceiling. Mitigation: Nebula
  is one binary from a well-audited upstream (`slackhq/nebula`)
  with no shell exec surface; the realistic attack vector is
  cert-validation bugs, not arbitrary file read. The added
  policy-module surface doesn't change the realistic risk.

**Post-1.0 trigger to author `nebula_mde_t`:** if Nebula upstream
gains a `--exec-hook` or shell-out feature, or if MDE-specific
mount points (Gluster bricks under `/var/lib/gluster/`) become
network-readable in error paths.

---

## 5. Threat model — flat trust inside the mesh

The platform is locked to **flat trust inside the mesh**
([[project_open_mesh_directive]], CLAUDE.md §0.14 master rule,
AI_GOVERNANCE.md §7 Q51 + Q54 + Q56). Every enrolled peer
fully trusts every other peer. The justification:

1. **The deployment profile is one operator + their peers.**
   8-peer cap (Q3 + Q22); the operator owns every peer in the
   mesh. Inter-peer ACLs would be the operator gatekeeping their
   own devices.
2. **The passcode is the operator's master credential.** Single,
   never rotates (Q51). Compromise of the passcode = compromise
   of the mesh. There is no second factor between mesh peers.
3. **`{{exec}}` in Bus templates is unrestricted** (Q56). Any
   peer can publish a Bus message that executes shell on any
   subscriber. This is a feature, not a bug — the operator wants
   their own peers to be able to do this. The mitigation is the
   passcode + Nebula transport.
4. **All-peer audit subscription by default** (Q54). Every peer
   sees every audit-log entry. The mesh is its own auditor;
   there is no privileged-peer visibility.

**What this posture is NOT designed for:**

- **Multi-tenant deployments.** Two operators sharing a mesh
  who don't fully trust each other is out of scope.
- **Adversary-on-mesh.** If an attacker gets the passcode + a
  signed cert, they are a full peer with full access. There is
  no inner perimeter.
- **Internal misuse / insider threat.** No audit redaction, no
  least-privilege between peers, no separation of duties.

**What this posture IS designed for:**

- **Adversary off-mesh.** Nebula's mutual-TLS handshake + the
  CA-issued cert per peer prevent unauthorized join. The
  passcode gate at birthright pairing prevents drive-by
  enrollment.
- **Lost / stolen peer.** CA revoke + ban-list (Q53) refuses
  re-join even with correct passcode. The Nebula CA on the
  lighthouse is the authority that revokes.
- **Local network adversary.** All inter-peer traffic rides
  Nebula (mutual-TLS, AEAD-encrypted UDP). LAN sniffing
  reveals only encrypted Nebula packets to the lighthouse's
  underlay IP.
- **Public internet adversary.** Bind-scope (CLAUDE.md §0.7
  gate #10) keeps every MDE listener on the Nebula overlay
  interface. Public scanning sees only UDP/4242 + TCP/443
  on lighthouses; everything else is overlay-only.

---

## 6. The intra-mesh boundary list

These are the security boundaries the platform actually enforces.
A reviewer asking "where is the trust boundary?" should be
pointed at this list:

| # | Boundary | Mechanism | Lint gate |
|---|---|---|---|
| 1 | Mesh enrollment | Single 16-char passcode + Nebula CA cert mint | n/a (operator-typed at pairing) |
| 2 | Underlay → overlay traffic | Nebula transport encryption (mutual-TLS + AEAD UDP) | n/a (data plane) |
| 3 | Listener public exposure | Bind-scope: every MDE listener on `nebula1` interface or overlay IP | `install-helpers/lint-public-ports.sh` (§0.7 #10) |
| 4 | D-Bus surface | Only FDO interop (`org.freedesktop.*`) survives 1.0; all MDE-internal IPC routes through Bus | `install-helpers/lint-dbus-shape.sh` (§0.7 #8) |
| 5 | Process privilege ceiling | systemd `CapabilityBoundingSet` + `NoNewPrivileges` per unit | n/a (unit-file review) |
| 6 | Operator-UID isolation | Every component runs as `$UID`; system services use ambient capabilities only | n/a (architectural) |
| 7 | New public-port surface | TUNE-3 + §0.8 gate #8: security review notes paragraph required on new ports | §0.8 #8 (Definition of Done) |
| 8 | Template-exec scope | `{{exec}}` runs as operator UID; same isolation as the operator's shell | n/a (Q56 flat-trust choice) |
| 9 | Phone trust | Phone gets full Nebula peer-hood per Q23 (25-Q); cert-pinned + bus-only + GFS-via-KDC2-drop | post-1.0 PHONE-NEBULA-PEER epic |
| 10 | Federation between meshes | OOB passcode + symmetric subscribe-only grant (Q35 + Q55) | post-1.0 BUS-7.7-FED epic |

The bind-scope lint (#3) is the load-bearing gate that prevents
the most common regression: a developer adding a new daemon that
binds `0.0.0.0` instead of the overlay IP. Snapshot-allow-list at
`install-helpers/lint-public-ports.sh:48` documents every existing
public-bind exception with its security rationale comment.

---

## 7. Defense-in-depth gaps documented as deliberate

These are the security controls MDE **does not** implement, with
the deliberate-choice rationale:

| Control NOT implemented | Why |
|---|---|
| AppArmor profiles | Fedora is SELinux, not AppArmor. Profile authoring is per-component (see §2 deferral). |
| Per-component SELinux types | Per §2: marginal value under user-UID isolation; trigger is multi-user-on-one-peer evolution. |
| Sandboxing per worker (bubblewrap / firejail) | Workers share-fate with the operator session by design (§3). |
| Encrypted-at-rest GFS bricks | Bricks live on `/var/lib/gluster/` under the host filesystem; rely on operator-configured LUKS for at-rest. |
| Audit-log retention enforcement | Bus audit log lives on GFS (`~/.local/share/mde/bus/audit/<date>.jsonl`); retention via BUS-1.9 quota. |
| Network-layer ACLs between peers | Q56 + Q51 — flat trust by design. |
| 2FA / hardware key on passcode | Single-credential model is the lock (Q51). Hardware-token enrollment is post-1.0. |
| Rate-limiting on Bus publish | Q56 — same trust assumption. Quota at BUS-1.9 (500 MB soft / 2 GB hard) is the only ceiling. |
| Per-peer audit-log redaction | Q54 — all-peer visibility is a feature, not a bug. |

Each row above is a deliberate trade-off. The platform's
"Secure, Simple, Centerless Workgroup" master rule trades
defense-in-depth against multi-tenant adversaries for **simplicity
of the single-operator deployment**. The 8-peer cap + the
passcode-is-master credential model + the flat-trust mesh form a
coherent posture: every operator-owned peer is equally privileged;
the boundary is the mesh perimeter, not internal partitions.

---

## 8. Cross-references

- **AI_GOVERNANCE.md §7** (Trust + security) — locks Q51..Q60 that
  this doc expands.
- **AI_GOVERNANCE.md §13.2** (Document map) — this doc is the §7
  expansion that closes the §11.5 gap noted in
  AI_PLATFORM_REFERENCE.md.
- **CLAUDE.md §0.7** — pre-commit gates that enforce the bind-scope
  + D-Bus shape + voice-tone boundaries.
- **CLAUDE.md §0.8** — Definition of Done gate #8 (security-review
  notes paragraph) on new public-port + new D-Bus method + new
  `{{exec}}` Tera template surfaces.
- **CLAUDE.md §0.16** — platform feature lock until next named cut;
  this doc lands during the EPIC-TUNING-25Q operator-issued lift.
- **`docs/design/v2.5-nebula-fabric.md`** — Nebula CA + cert
  lifecycle, lighthouse model.
- **`docs/design/v6.x-mackes-bus.md`** §10 — `{{exec}}` template
  surface + flat-trust amplifier discussion.
- **[[project_open_mesh_directive]]** — the operator's
  flat-trust / single-passcode / no-per-node-ACLs directive that
  anchors §5 + §6.
