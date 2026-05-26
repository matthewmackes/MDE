# Cap overrides

**Lock date:** 2026-05-26 (TUNE-11, 25-Q tuning survey aftermath)
**Authority:** AI_GOVERNANCE.md §7 (Q3 + Q22 8-peer cap) + CLAUDE.md
§0.14 hierarchy. This doc names the operator-facing escape hatch
when the cap legitimately needs to be exceeded.

The 8-peer cap is a deliberate lock. The override is for documented
exceptions, not for routine bypass.

---

## 1. Why the cap exists

[`crates/mackesd/src/ca/sign.rs`](../../crates/mackesd/src/ca/sign.rs)
sets `pub const MAX_PEER_CAP: u32 = 8`. The CA's
`sign_pending_csr` enforces it before signing a peer cert
(TUNE-11). Locked by:

- **Q3 (100-Q tightening survey, 2026-05-25)** — tightens the
  earlier 16-peer aspirational sizing to a small-circle 8-peer
  cap. Three system properties depend on the cap:
  - **Gluster replica cost.** Full-mesh replicated `mesh-home`
    means N writes per write; at N > 8 the rebalance + heal
    timing on a typical home-office LAN degrades user-visible.
  - **Bus broker mesh.** Per-peer ntfy broker over Nebula with
    gossip discovery + audit; the topology scales at N² edges
    in the worst case. 8 stays inside the LAN's bandwidth + the
    operator's mental model.
  - **Attendance + leader election.** mackesd's QNM-Shared
    lockfile + the BUS quorum heuristics assume an operator who
    can spot a dead peer by sight. 8 is a number the operator
    knows by name.
- **Q22 (100-Q survey)** — re-affirms the cap holds after BUS +
  GFS folded in, and explicitly removes federated peers from the
  count (federation is the post-cap scale path, not within-mesh
  growth).
- **Q3 of the 25-Q tuning survey (2026-05-26)** — re-affirms the
  cap + lifts it into runtime enforcement via TUNE-11.

§11 1.0 roadmap item #10 + the EPIC-MASTER-3 close-out (worklist
line 1474) confirm the cap is consistent across every layer:
docs, memory, design specs, code-level extension allocations.

---

## 2. What the override does

`mackesd ca sign-csr <node-id> --override-cap`:

1. Bypasses the
   [`SignCsrError::PeerCapReached`](../../crates/mackesd/src/nebula_enroll.rs)
   gate in `sign_pending_csr`.
2. Emits a `tracing::warn!` line at
   `target = "mackesd::cap_override"` with structured fields:
   - `event = "cap.override.engaged"`
   - `peer_id = <node-id>`
   - `mesh_id = <mesh-id>`
   - `current = <count-when-override-fired>`
   - `cap = 8`
3. Prints a clear stderr line at sign time:
   ```
   TUNE-11 OVERRIDE ENGAGED: signed peer:<node-id> past the 8-peer cap.
   Audit-log entry written to the journal under `mackesd::cap_override`.
   Document the exception in docs/design/cap-overrides.md.
   ```

The override does **not** suppress the next sign's gate — every
subsequent sign requires its own `--override-cap` flag. There is
no "always override" mode by design (no `MACKES_DISABLE_CAP` env
var, no config-file setting).

The watcher worker
([`crates/mackesd/src/workers/nebula_csr_watcher.rs`](../../crates/mackesd/src/workers/nebula_csr_watcher.rs))
NEVER auto-overrides — it always passes `allow_override = false`
so background CSR signing never silently bypasses the cap. The
only path past the gate is the explicit operator CLI flag.

---

## 3. What an audit-log entry looks like

```
WARN mackesd::cap_override event="cap.override.engaged" peer_id="peer:ninth"
     mesh_id="mesh-fedora" current=8 cap=8
     TUNE-11: signing peer past the 8-peer cap by operator override
```

Operators harvest via `journalctl -u mackesd | grep cap.override`
(or the per-user variant for `systemd --user` deployments). The
JSON-structured fields make it grep-friendly + tooling-ready for
future Bus integration.

**Future-work cross-ref:** when BUS-7 federation lands, the
override should also publish to `mesh/sec/cap-override` per peer
so the entry surfaces in Workbench → Mesh → Bus → Audit alongside
every other security-relevant event. The journal entry is the
1.0 truth; the Bus mirror is a 1.x convenience.

---

## 4. When the override is legitimate

The exception is **rare**. Document any real use here so future
operators + security reviewers understand why the cap was
exceeded:

| Date | Operator | Peer | Reason |
|---|---|---|---|
| _(no overrides recorded yet)_ | | | |

If you engage the override, add a row above with:
- ISO date of the override.
- Operator identity (matthewmackes / etc.).
- The peer's `node_id` that was admitted past the cap.
- One-sentence reason ("temporary lab peer for X experiment, expired YYYY-MM-DD";
  "guest contractor under NDA for Y review, removed YYYY-MM-DD").

After the temporary peer leaves the mesh, revoke its cert via the
standard CA revoke path (`mackesd ca revoke <node-id>`) so the
active count drops back inside the cap.

---

## 5. When the override is NOT the right tool

- **"I want a 9th peer permanently."** Don't override. Lift Q3
  via an operator-typed `lift the lock for X` + an N-Q survey
  per CLAUDE.md §0.16 platform feature lock. The cap is the
  capability ceiling for 1.0; raising it changes the platform's
  design assumptions about gluster replica cost + Bus topology
  + attendance election.
- **"Federation between meshes."** Federation is the locked path
  past the cap (Q35 + Q55 + BUS-7.7-FED). Federated peers
  DON'T count against the cap by design (Q22). Use
  `docs/design/v1.0-federation-pairing.md` (TUNE-15) when it
  ships.
- **"Add the phone as the 9th peer."** Phones get full Nebula
  peer-hood per Q23 of the 25-Q survey (TUNE-16 +
  PHONE-NEBULA-PEER epic). Phone counts against the cap (4
  desktops + 1 phone = 5 of 8 — well under). Use the phone-pair
  UI when PHONE-NEBULA-PEER ships.
- **"Temporary CI runner peer."** Don't enroll into the
  production mesh. Stand up a separate mesh (mesh-id =
  `mesh-ci-<run>`) with its own CA + passcode. CI never shares a
  passcode with the operator's working mesh.

---

## 6. Removing the override path

The override flag exists because Q3 anticipates a small number of
legitimate exceptions. If the override hasn't been used by the
1.5 milestone, evaluate whether to retire the `--override-cap`
flag entirely + replace it with the Q3-amend survey path. The
cleaner runtime is "no escape hatch" — overrides exist only as
long as they're needed.

---

## 7. Cross-references

- **`crates/mackesd/src/ca/sign.rs`** — `MAX_PEER_CAP` constant +
  `count_active_peers()` helper.
- **`crates/mackesd/src/nebula_enroll.rs`** —
  `sign_pending_csr()` cap check + `SignCsrError::PeerCapReached`
  variant.
- **`crates/mackesd/src/bin/mackesd.rs`** — `sign-csr
  --override-cap` CLI flag.
- **AI_GOVERNANCE.md §7 + §11** — the cap lock + post-1.0 path.
- **CLAUDE.md §0.16** — platform feature lock + the operator
  override required to raise Q3.
- **`docs/design/security-posture.md`** §5 — threat model that
  rationalizes the small-circle cap.
