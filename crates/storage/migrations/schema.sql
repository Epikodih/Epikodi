-- =====================================================================
--  Media center : schéma SQLite (base locale), version 3 : simple
--
--  Idée générale :
--   * media          = tout ce qu'on AFFICHE dans le catalogue (cartes, listes)
--   * media_details  = la fiche détaillée (JSON), chargée au clic
--   * Le scan remplit media et media_details. Rien d'autre à maintenir.
--   * Un seul type d'objet pour tout : film, série, saison, épisode, artiste,
--     album, morceau, podcast. Les liens se font avec parent_id.
--
--  À exécuter à chaque connexion : PRAGMA foreign_keys = ON;
-- =====================================================================

PRAGMA foreign_keys = ON;
PRAGMA user_version = 3;

-- ---------------------------------------------------------------------
-- PROFILS
--   restreint     : accès libre, voit seulement age_rating <= age
--   non restreint : le PIN global est demandé pour l'ouvrir
--   Le premier profil créé est non restreint et définit le PIN.
--   preferences (JSON) : {"theme":"default","text_scale":1.0,"reduce_motion":false,
--     "subtitles":{"size":1.0,"color":"#FFFFFF","background":"#000000","delay_ms":0},
--     "audio_lang":"fr","subtitle_lang":"fr","speech":false}
-- ---------------------------------------------------------------------
CREATE TABLE user (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT    NOT NULL UNIQUE,
    icon          TEXT,
    language      TEXT    NOT NULL DEFAULT 'fr',
    age           INTEGER NOT NULL CHECK (age BETWEEN 0 AND 120),
    is_restricted INTEGER NOT NULL DEFAULT 0 CHECK (is_restricted IN (0, 1)),
    preferences   TEXT    NOT NULL DEFAULT '{}' CHECK (json_valid(preferences))
);

-- ---------------------------------------------------------------------
-- TÉLÉPHONES APPAIRÉS (seul le hash du jeton est stocké)
-- ---------------------------------------------------------------------
CREATE TABLE device (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    paired_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TEXT                       -- NULL = autorisé
);

-- ---------------------------------------------------------------------
-- SOURCES (dossier local ou partage réseau)
-- ---------------------------------------------------------------------
CREATE TABLE source (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    name                  TEXT NOT NULL,
    protocol              TEXT NOT NULL CHECK (protocol IN ('local', 'smb', 'dlna', 'nfs', 'webdav')),
    uri                   TEXT NOT NULL UNIQUE,   -- file:///..., smb://nas/films
    credentials_encrypted BLOB,                   -- jamais en clair
    status                TEXT NOT NULL DEFAULT 'unknown'
                          CHECK (status IN ('online', 'offline', 'error', 'unknown')),
    last_scan_at          TEXT
);

-- ---------------------------------------------------------------------
-- MEDIA : ce qu'on affiche dans le catalogue
--
--  kind            rangé sous (parent_id)        sort_number
--  --------------  ----------------------------  -------------------
--  movie           (rien)
--  series          (rien)
--  season          une series                    numéro de saison
--  episode         une season                    numéro d'épisode
--  artist          (rien)
--  album           un artist                     (année ou ordre)
--  track           un album                      numéro de piste
--  podcast         (rien) ; uri = URL du flux
--  podcast_episode un podcast                    (ordre de parution)
--
--  Les "feuilles" (movie, episode, track, podcast_episode) ont un uri :
--  c'est le fichier ou l'URL à lire. Les conteneurs (series, season, artist,
--  album) n'ont pas de fichier, donc uri est NULL.
-- ---------------------------------------------------------------------
CREATE TABLE media (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id    INTEGER REFERENCES media(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK (kind IN
                 ('movie', 'series', 'season', 'episode',
                  'artist', 'album', 'track', 'podcast', 'podcast_episode')),

    -- Affichage
    title        TEXT NOT NULL,                -- toujours rempli (nom de fichier au départ)
    year         INTEGER,
    poster_path  TEXT,                         -- image dans le cache
    duration_s   INTEGER,
    sort_number  INTEGER,
    age_rating   INTEGER CHECK (age_rating IN (0, 10, 12, 16, 18)),  -- NULL = inconnue (masqué aux restreints)
    match_status TEXT NOT NULL DEFAULT 'unmatched'
                 CHECK (match_status IN ('unmatched', 'auto', 'manual')),  -- manual = ne pas écraser

    -- Fichier (scan)
    source_id    INTEGER REFERENCES source(id) ON DELETE SET NULL,  -- NULL = podcast distant
    uri          TEXT UNIQUE,
    parent_dir   TEXT,                         -- pour le scan ciblé d'un dossier
    size_bytes   INTEGER,
    modified_at  TEXT,
    fingerprint  TEXT,                         -- retrouver un fichier déplacé
    status       TEXT NOT NULL DEFAULT 'available' CHECK (status IN ('available', 'missing')),
    last_seen_at TEXT,
    added_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CHECK (kind NOT IN ('movie', 'episode', 'track', 'podcast_episode') OR uri IS NOT NULL)
);
CREATE INDEX idx_media_parent     ON media(parent_id, sort_number);
CREATE INDEX idx_media_kind       ON media(kind, status);
CREATE INDEX idx_media_title      ON media(title);
CREATE INDEX idx_media_parent_dir ON media(parent_dir);
CREATE INDEX idx_media_added_at   ON media(added_at);

-- ---------------------------------------------------------------------
-- MEDIA_DETAILS : la fiche détaillée, en JSON
--  Une ligne au plus par media. Pas de ligne = pas encore enrichi.
--  Contenu libre selon le type, par exemple :
--  {
--    "overview": "...", "original_title": "...", "tagline": "...",
--    "release_date": "2020-05-01", "backdrop": "cache/xxx.jpg", "rating": 7.8,
--    "genres": ["Drame", "Science-fiction"],
--    "cast": [{"name": "...", "role": "...", "character": "..."}],
--    "lyrics": "...",
--    "tech": {"container": "mkv", "video_codec": "h264", "audio_codec": "aac",
--             "width": 1920, "height": 1080, "bitrate": 5200000},
--    "audio_tracks": [{"lang": "fr", "codec": "aac", "audio_description": false}],
--    "subtitles": [{"lang": "fr", "format": "srt", "embedded": false, "uri": "..."}],
--    "external_ids": {"tmdb": 123, "musicbrainz": "..."},
--    "provider": "tmdb"
--  }
-- ---------------------------------------------------------------------
CREATE TABLE media_details (
    media_id INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
    details  TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(details))
);

-- ---------------------------------------------------------------------
-- DONNÉES PAR PROFIL
-- ---------------------------------------------------------------------
-- Historique, reprise de lecture, vu / non vu
CREATE TABLE watch_progress (
    user_id        INTEGER NOT NULL REFERENCES user(id)  ON DELETE CASCADE,
    media_id       INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    position_s     INTEGER NOT NULL DEFAULT 0,
    is_watched     INTEGER NOT NULL DEFAULT 0 CHECK (is_watched IN (0, 1)),
    last_played_at TEXT,
    PRIMARY KEY (user_id, media_id)
);
CREATE INDEX idx_watch_progress_recent ON watch_progress(user_id, last_played_at DESC);

-- Favoris : tout est un media (film, série, album, artiste, podcast...)
CREATE TABLE favorite (
    user_id  INTEGER NOT NULL REFERENCES user(id)  ON DELETE CASCADE,
    media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, media_id)
);

CREATE TABLE playlist (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES user(id) ON DELETE CASCADE,
    name    TEXT NOT NULL,
    UNIQUE (user_id, name)
);

CREATE TABLE playlist_item (
    playlist_id INTEGER NOT NULL REFERENCES playlist(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    media_id    INTEGER NOT NULL REFERENCES media(id)    ON DELETE CASCADE,
    PRIMARY KEY (playlist_id, position)
);

-- ---------------------------------------------------------------------
-- FILE D'ATTENTE (une seule, globale)
-- ---------------------------------------------------------------------
CREATE TABLE queue_item (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    position INTEGER NOT NULL
);
CREATE INDEX idx_queue_position ON queue_item(position);

-- ---------------------------------------------------------------------
-- ÉTAT DE L'APPLICATION (une seule ligne, id = 1)
--   parental_pin_hash : PIN parental global, hashé (NULL tant qu'il n'est pas défini)
-- ---------------------------------------------------------------------
CREATE TABLE app_state (
    id                    INTEGER PRIMARY KEY CHECK (id = 1),
    parental_pin_hash     TEXT,
    active_user_id        INTEGER REFERENCES user(id) ON DELETE SET NULL,
    current_queue_item_id INTEGER REFERENCES queue_item(id) ON DELETE SET NULL,
    volume                INTEGER NOT NULL DEFAULT 80 CHECK (volume BETWEEN 0 AND 100),
    shuffle               INTEGER NOT NULL DEFAULT 0 CHECK (shuffle IN (0, 1)),
    repeat_mode           TEXT NOT NULL DEFAULT 'off' CHECK (repeat_mode IN ('off', 'one', 'all'))
);
INSERT INTO app_state (id) VALUES (1);

-- ---------------------------------------------------------------------
-- EXTENSIONS
--   permissions (JSON) : {"network": true, "library.read": false, "storage": true}
--                        (true = acceptée par l'utilisateur)
--   settings    (JSON) : réglages de l'extension
-- ---------------------------------------------------------------------
CREATE TABLE extension (
    id           TEXT PRIMARY KEY,               -- identifiant du manifeste
    name         TEXT NOT NULL,
    version      TEXT NOT NULL,
    type         TEXT NOT NULL CHECK (type IN ('source', 'metadata', 'widget')),
    install_path TEXT NOT NULL,
    enabled      INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    status       TEXT NOT NULL DEFAULT 'ok' CHECK (status IN ('ok', 'crashed', 'stopped')),
    last_error   TEXT,
    permissions  TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(permissions)),
    settings     TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(settings))
);

-- =====================================================================
-- REQUÊTES TYPES
-- =====================================================================
-- Catalogue des films visibles par un profil :
--   SELECT m.id, m.title, m.year, m.poster_path FROM media m, user u
--   WHERE u.id = :user_id AND m.kind = 'movie' AND m.status = 'available'
--     AND (u.is_restricted = 0 OR (m.age_rating IS NOT NULL AND m.age_rating <= u.age))
--   ORDER BY m.title;
--
-- Saisons d'une série :   SELECT * FROM media WHERE parent_id = :serie_id ORDER BY sort_number;
-- Épisodes d'une saison : SELECT * FROM media WHERE parent_id = :saison_id ORDER BY sort_number;
-- Fiche au clic :         SELECT m.*, d.details FROM media m
--                         LEFT JOIN media_details d ON d.media_id = m.id WHERE m.id = :id;
-- Un champ du JSON :      SELECT json_extract(details, '$.overview') FROM media_details WHERE media_id = :id;
-- Reprise de lecture :    SELECT * FROM watch_progress
--                         WHERE user_id = :u AND is_watched = 0 AND position_s > 0
--                         ORDER BY last_played_at DESC LIMIT 20;
-- Scan ciblé d'un dossier : SELECT * FROM media WHERE parent_dir = :dossier;
