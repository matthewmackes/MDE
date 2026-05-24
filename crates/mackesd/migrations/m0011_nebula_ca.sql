-- mackesd m0011_nebula_ca — v2.5 Nebula fabric rebuild (locked 2026-05-23)
--
-- Adds the two CA tables the v2.5 Nebula fabric needs. Strictly
-- additive: the existing Phase 12 + v2.0.0 tables in `0001_init.sql`
-- and `0002_settings_session.sql` are unchanged.
--
-- Two tables land here:
--
--   nebula_ca           one row per (mesh_id, epoch) — the
--                       self-signed Nebula CA cert that signs every
--                       peer's host cert. Epoch 0 is the initial
--                       mint; failover bumps the epoch and seals a
--                       fresh CA (NF-2.5).
--
--   nebula_peer_certs   one row per (node_id, epoch) — a peer's
--                       Nebula host cert signed by the CA at
--                       `epoch`. Carries the overlay IP allocated
--                       from the mesh CIDR (10.42.0.0/16 default).
--
-- Per the v2.5 lock (Q3), the CA private key NEVER lives in SQL —
-- it's sealed at `/var/lib/mackesd/nebula-ca/ca.key` (mode 0600
-- root:root). Only the public PEM-encoded cert lands here so peers
-- can read the active CA without root.

CREATE TABLE nebula_ca (
    mesh_id      TEXT    NOT NULL,
    epoch        INTEGER NOT NULL,
    ca_cert_pem  TEXT    NOT NULL,
    created_at   INTEGER NOT NULL,
    retired_at   INTEGER,
    PRIMARY KEY (mesh_id, epoch)
);
CREATE INDEX idx_nebula_ca_active ON nebula_ca(mesh_id) WHERE retired_at IS NULL;

CREATE TABLE nebula_peer_certs (
    node_id     TEXT    NOT NULL,
    epoch       INTEGER NOT NULL,
    cert_pem    TEXT    NOT NULL,
    overlay_ip  TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    expires_at  INTEGER NOT NULL,
    revoked_at  INTEGER,
    PRIMARY KEY (node_id, epoch)
);
CREATE INDEX idx_nebula_peer_certs_active ON nebula_peer_certs(epoch) WHERE revoked_at IS NULL;
CREATE INDEX idx_nebula_peer_certs_overlay ON nebula_peer_certs(overlay_ip);
