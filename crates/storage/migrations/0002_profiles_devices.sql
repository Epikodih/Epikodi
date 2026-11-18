-- =====================================================================
--  0002 : PIN par compte, appareils (installation, expiration du jeton)
--
--  Règle les écarts entre le schéma v3 et le protocole (fixtures/README.md) :
--   * chaque compte non restreint a son propre PIN (haché) ;
--   * un appareil garde l'identifiant de son installation, sa plateforme,
--     l'expiration de son jeton et sa dernière activité.
--  app_state.parental_pin_hash n'est plus utilisé (gardé : on ne modifie
--  jamais une migration livrée).
-- =====================================================================

ALTER TABLE user ADD COLUMN pin_hash TEXT;

ALTER TABLE device ADD COLUMN install_id   TEXT;
ALTER TABLE device ADD COLUMN platform     TEXT;
ALTER TABLE device ADD COLUMN expires_at   TEXT;
ALTER TABLE device ADD COLUMN last_seen_at TEXT;
CREATE INDEX idx_device_install ON device(install_id);

-- Retrouver un fichier déplacé, retrouver un conteneur par titre.
CREATE INDEX idx_media_fingerprint ON media(fingerprint);
CREATE INDEX idx_media_source      ON media(source_id);
