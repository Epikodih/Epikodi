//! Reconnaissance des noms de fichiers.
//!
//! - Séries : `Serie.S01E02.mkv`, `serie 1x02.avi`, `Serie/Saison 1/Episode 2.mkv`...
//! - Films : titre et année (`Titre (2019)`, `Titre.2019.1080p.BluRay.x264-GRP`).
//! - Musique : tags audio, sinon `Artiste/Album/01 - Titre.mp3`.
//!
//! Ajouter une règle : écrire d'abord le cas dans les tests (`CAS_DIFFICILES`),
//! puis adapter la fonction concernée (`detect_episode`, `split_title_year`...).

use std::sync::OnceLock;

use regex::Regex;

use epikodi_domain::source::AudioTags;
pub use epikodi_domain::source::{AUDIO_EXTENSIONS, VIDEO_EXTENSIONS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Video,
    Audio,
    Other,
}

/// Type d'un fichier d'après son extension.
pub fn file_kind(path: &str) -> FileKind {
    let name = file_name(path);
    if name.starts_with('.') {
        return FileKind::Other; // fichiers cachés (._xxx de macOS...)
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some(e) if VIDEO_EXTENSIONS.contains(&e) => FileKind::Video,
        Some(e) if AUDIO_EXTENSIONS.contains(&e) => FileKind::Audio,
        _ => FileKind::Other,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Recognized {
    Movie {
        title: String,
        year: Option<i32>,
    },
    Episode {
        series: String,
        series_year: Option<i32>,
        season: i64,
        episode: i64,
        title: Option<String>,
    },
    Track {
        artist: String,
        album: String,
        title: String,
        track: Option<u32>,
        year: Option<i32>,
        duration_s: Option<i64>,
    },
}

pub const UNKNOWN_ARTIST: &str = "Artiste inconnu";
pub const UNKNOWN_ALBUM: &str = "Album inconnu";

/// Reconnaît un fichier à partir de son chemin relatif à la source.
/// `None` si ce n'est pas un média (sous-titre, image, texte...).
pub fn recognize(relative_path: &str, tags: Option<&AudioTags>) -> Option<Recognized> {
    match file_kind(relative_path) {
        FileKind::Video => Some(recognize_video(relative_path)),
        FileKind::Audio => Some(recognize_audio(relative_path, tags)),
        FileKind::Other => None,
    }
}

// ----------------------------------------------------------------- vidéo

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("regex valide"))
}

fn sxxeyy() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(
        &R,
        r"(?i)(?:^|[\s._\-\[(])s(\d{1,2})[\s._\-]?e(\d{1,3})(?:[\s._\-]?e\d{1,3})*",
    )
}

fn nxnn() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(
        &R,
        r"(?i)(?:^|[\s._\-\[(])(\d{1,2})x(\d{2,3})(?:$|[\s._\-\])])",
    )
}

fn season_dir() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"(?i)^(?:season|saison|series|s)[\s._\-]*(\d{1,2})$")
}

fn episode_in_season_dir() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(
        &R,
        r"(?i)^(?:e|ep|episode|épisode)?[\s._\-]*(\d{1,3})(?:$|[\s._\-]+(.*)$)",
    )
}

fn leading_tag() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"^\s*\[[^\]]*\]\s*")
}

fn year_token() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"^[\(\[]?((?:19|20)\d\d)[\)\]]?$")
}

/// Mots qui marquent la fin du titre (qualité, codec, langue, groupe...).
const JUNK: &[&str] = &[
    "2160p",
    "1080p",
    "1080i",
    "720p",
    "576p",
    "480p",
    "4k",
    "uhd",
    "hdr",
    "hdr10",
    "dv",
    "x264",
    "x265",
    "h264",
    "h265",
    "hevc",
    "avc",
    "xvid",
    "divx",
    "av1",
    "10bit",
    "bluray",
    "blu-ray",
    "bdrip",
    "brrip",
    "webrip",
    "web-dl",
    "webdl",
    "web",
    "dvdrip",
    "dvd",
    "hdtv",
    "hdrip",
    "remux",
    "aac",
    "ac3",
    "eac3",
    "dts",
    "truehd",
    "atmos",
    "mp3",
    "flac",
    "multi",
    "vostfr",
    "vf",
    "vff",
    "vfq",
    "french",
    "truefrench",
    "subfrench",
    "english",
    "vo",
    "proper",
    "repack",
    "extended",
    "remastered",
    "unrated",
    "directors",
    "uncut",
    "final",
    "limited",
    "internal",
    "complete",
    "hd",
    "sd",
];

fn is_junk(token: &str) -> bool {
    let t = token
        .trim_matches(|c: char| matches!(c, '[' | ']' | '(' | ')' | '{' | '}'))
        .to_ascii_lowercase();
    JUNK.contains(&t.as_str())
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn stem(path: &str) -> &str {
    let name = file_name(path);
    name.rsplit_once('.').map_or(name, |(s, _)| s)
}

/// Dossiers parents, du plus proche au plus lointain.
fn parent_dirs(path: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = path.split(['/', '\\']).filter(|p| !p.is_empty()).collect();
    parts.pop();
    parts.reverse();
    parts
}

fn tokens(s: &str) -> Vec<&str> {
    s.split(|c: char| c.is_whitespace() || c == '.' || c == '_')
        .filter(|t| !t.is_empty() && *t != "-")
        .collect()
}

/// Titre lisible : séparateurs → espaces, coupé au premier mot « technique ».
/// Garde un groupe de sortie (`-GRP`) hors du titre.
fn clean_title(s: &str) -> String {
    let s = leading_tag().replace(s, "");
    let mut out: Vec<&str> = Vec::new();
    for t in tokens(&s) {
        if is_junk(t) || t.starts_with('[') {
            break;
        }
        // « x264-GROUP » ou « BluRay-GROUP » : fin du titre.
        if let Some((head, _)) = t.split_once('-') {
            if is_junk(head) {
                break;
            }
        }
        out.push(t);
    }
    finish_title(&out.join(" "))
}

fn finish_title(s: &str) -> String {
    let s = s
        .trim()
        .trim_matches(|c: char| c == '-' || c == '–' || c == ',' || c.is_whitespace())
        .to_string();
    // Tout en minuscules : on met une capitale à chaque mot.
    if !s.is_empty() && !s.chars().any(char::is_uppercase) {
        s.split(' ')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect::<String>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        s
    }
}

/// Sépare titre et année : la DERNIÈRE année plausible qui n'est pas le
/// premier mot (`2001 A Space Odyssey 1968` → titre `2001 A Space Odyssey`).
fn split_title_year(s: &str) -> (String, Option<i32>) {
    let s = leading_tag().replace(s, "").into_owned();
    let toks = tokens(&s);
    let year_idx = toks
        .iter()
        .enumerate()
        .skip(1)
        .rev()
        .find(|(_, t)| year_token().is_match(t))
        .map(|(i, _)| i);
    match year_idx {
        Some(i) => {
            let year = year_token()
                .captures(toks[i])
                .and_then(|c| c[1].parse().ok());
            let title = clean_title(&toks[..i].join(" "));
            if title.is_empty() {
                (clean_title(&s), None)
            } else {
                (title, year)
            }
        }
        None => (clean_title(&s), None),
    }
}

fn is_generic_dir(d: &str) -> bool {
    matches!(
        d.to_lowercase().as_str(),
        "series"
            | "séries"
            | "serie"
            | "série"
            | "tv"
            | "tv shows"
            | "shows"
            | "videos"
            | "vidéos"
            | "films"
            | "movies"
            | "media"
            | "médias"
    )
}

fn series_from_dirs(dirs: &[&str]) -> Option<(String, Option<i32>)> {
    dirs.iter()
        .find(|d| !season_dir().is_match(d) && !is_generic_dir(d))
        .map(|d| split_title_year(d))
        .filter(|(t, _)| !t.is_empty())
}

fn detect_episode(path: &str) -> Option<Recognized> {
    let stem = stem(path);
    let dirs = parent_dirs(path);

    let found = sxxeyy()
        .captures(stem)
        .or_else(|| nxnn().captures(stem))
        .map(|c| {
            let m = c.get(0).unwrap();
            (
                c[1].parse::<i64>().ok(),
                c[2].parse::<i64>().ok(),
                &stem[..m.start()],
                &stem[m.end()..],
            )
        });

    if let Some((Some(season), Some(episode), prefix, rest)) = found {
        let (series, series_year) = match split_title_year(prefix) {
            (t, y) if !t.is_empty() => (t, y),
            _ => series_from_dirs(&dirs)?,
        };
        let title = Some(clean_title(rest)).filter(|t| !t.is_empty());
        return Some(Recognized::Episode {
            series,
            series_year,
            season,
            episode,
            title,
        });
    }

    // Rangé dans un dossier « Saison N » : le numéro est dans le nom.
    let season_idx = dirs.iter().position(|d| season_dir().is_match(d))?;
    let season: i64 = season_dir().captures(dirs[season_idx])?[1].parse().ok()?;
    let caps = episode_in_season_dir().captures(stem.trim())?;
    let episode: i64 = caps[1].parse().ok()?;
    let title = caps
        .get(2)
        .map(|t| clean_title(t.as_str()))
        .filter(|t| !t.is_empty());
    let (series, series_year) = series_from_dirs(&dirs[season_idx + 1..])?;
    Some(Recognized::Episode {
        series,
        series_year,
        season,
        episode,
        title,
    })
}

fn recognize_video(path: &str) -> Recognized {
    if let Some(ep) = detect_episode(path) {
        return ep;
    }
    let (title, year) = split_title_year(stem(path));
    let title = if title.is_empty() {
        stem(path).to_string()
    } else {
        title
    };
    Recognized::Movie { title, year }
}

// ----------------------------------------------------------------- audio

fn track_prefix() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    // « 01 - Titre », « 1. Titre », « 01_Titre », « 101 Titre » (disque 1)
    re(&R, r"^(\d{1,3})[\s._\-]+(.+)$")
}

fn non_empty(s: Option<&String>) -> Option<String> {
    s.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn recognize_audio(path: &str, tags: Option<&AudioTags>) -> Recognized {
    let dirs = parent_dirs(path);
    let stem = stem(path);
    let tags = tags.cloned().unwrap_or_default();

    // Depuis le nom : « 01 - Titre » ou « Artiste - Titre ».
    let (path_track, mut path_title, path_artist) = match track_prefix().captures(stem) {
        Some(c) => (
            c[1].parse::<u32>()
                .ok()
                .map(|n| if n > 100 { n % 100 } else { n }),
            clean_audio(&c[2]),
            None,
        ),
        None => match stem.split_once(" - ") {
            Some((a, t)) => (None, clean_audio(t), Some(clean_audio(a))),
            None => (None, clean_audio(stem), None),
        },
    };
    if path_title.is_empty() {
        path_title = stem.to_string();
    }

    let album = non_empty(tags.album.as_ref())
        .or_else(|| {
            dirs.first()
                .filter(|d| !is_generic_audio_dir(d))
                .map(|d| clean_audio(d))
        })
        .unwrap_or_else(|| UNKNOWN_ALBUM.to_string());
    let artist = non_empty(tags.artist.as_ref())
        .or(path_artist)
        .or_else(|| {
            dirs.get(1)
                .filter(|d| !is_generic_audio_dir(d))
                .map(|d| clean_audio(d))
        })
        .unwrap_or_else(|| UNKNOWN_ARTIST.to_string());

    Recognized::Track {
        artist,
        album,
        title: non_empty(tags.title.as_ref()).unwrap_or(path_title),
        track: tags.track_number.or(path_track),
        year: tags.year,
        duration_s: tags.duration_s,
    }
}

fn clean_audio(s: &str) -> String {
    s.replace('_', " ").trim().to_string()
}

fn is_generic_audio_dir(d: &str) -> bool {
    matches!(
        d.to_lowercase().as_str(),
        "music" | "musique" | "audio" | "mp3" | "flac"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movie(t: &str, y: Option<i32>) -> Recognized {
        Recognized::Movie {
            title: t.into(),
            year: y,
        }
    }

    fn ep(series: &str, s: i64, e: i64, title: Option<&str>) -> Recognized {
        Recognized::Episode {
            series: series.into(),
            series_year: None,
            season: s,
            episode: e,
            title: title.map(Into::into),
        }
    }

    /// Jeu de noms de fichiers difficiles (mal formatés, ambigus).
    #[test]
    fn cas_difficiles() {
        let cases: Vec<(&str, Recognized)> = vec![
            // films
            (
                "Sintel.2010.1080p.BluRay.x264-GRP.mkv",
                movie("Sintel", Some(2010)),
            ),
            ("Big_Buck_Bunny_1080p.mp4", movie("Big Buck Bunny", None)),
            (
                "Elephants Dream (2006).webm",
                movie("Elephants Dream", Some(2006)),
            ),
            ("tears_of_steel_720p.mkv", movie("Tears Of Steel", None)),
            ("mon_film_2019_final.mp4", movie("Mon Film", Some(2019))),
            (
                "2001.A.Space.Odyssey.1968.mkv",
                movie("2001 A Space Odyssey", Some(1968)),
            ),
            ("1917.mkv", movie("1917", None)),
            (
                "Blade Runner 2049 (2017).mkv",
                movie("Blade Runner 2049", Some(2017)),
            ),
            (
                "[Team] Cosmos Laundromat [2015] [720p].mp4",
                movie("Cosmos Laundromat", Some(2015)),
            ),
            (
                "The.Matrix.1999.REMASTERED.MULTI.2160p.mkv",
                movie("The Matrix", Some(1999)),
            ),
            ("Films/Spring - 2019.mp4", movie("Spring", Some(2019))),
            (
                "Caminandes 3 Llamigos.mov",
                movie("Caminandes 3 Llamigos", None),
            ),
            // séries
            ("Serie.S01E02.mkv", ep("Serie", 1, 2, None)),
            (
                "Série Démo - S01E03 - Le titre.mkv",
                ep("Série Démo", 1, 3, Some("Le titre")),
            ),
            ("serie.demo.1x04.hdtv.mkv", ep("Serie Demo", 1, 4, None)),
            ("série démo s2e5.mkv", ep("Série Démo", 2, 5, None)),
            (
                "Séries/Série Démo/Saison 2/Episode 06.mkv",
                ep("Série Démo", 2, 6, None),
            ),
            (
                "Série Démo/S02/E07 - Fin.mkv",
                ep("Série Démo", 2, 7, Some("Fin")),
            ),
            (
                "Série Démo/Season 1/s01e08.mkv",
                ep("Série Démo", 1, 8, None),
            ),
            (
                "Show.Name.S01E01E02.720p.WEB-DL.mkv",
                ep("Show Name", 1, 1, None),
            ),
            (
                "Serie_S03.E10_Titre_Fou.mp4",
                ep("Serie", 3, 10, Some("Titre Fou")),
            ),
        ];
        for (path, expected) in cases {
            assert_eq!(recognize(path, None).unwrap(), expected, "fichier : {path}");
        }
    }

    #[test]
    fn annee_de_la_serie_retiree_du_nom() {
        assert_eq!(
            recognize("Show.Name.2021.S01E01.1080p.mkv", None).unwrap(),
            Recognized::Episode {
                series: "Show Name".into(),
                series_year: Some(2021),
                season: 1,
                episode: 1,
                title: None,
            }
        );
    }

    #[test]
    fn resolution_n_est_pas_un_episode() {
        assert_eq!(
            recognize("Film.1920x1080.mkv", None).unwrap(),
            movie("Film 1920x1080", None)
        );
    }

    #[test]
    fn musique_depuis_les_tags() {
        let tags = AudioTags {
            title: Some("Morceau".into()),
            artist: Some("Artiste Démo".into()),
            album: Some("Album Démo".into()),
            track_number: Some(3),
            year: Some(2020),
            duration_s: Some(200),
        };
        assert_eq!(
            recognize("n'importe/quoi.mp3", Some(&tags)).unwrap(),
            Recognized::Track {
                artist: "Artiste Démo".into(),
                album: "Album Démo".into(),
                title: "Morceau".into(),
                track: Some(3),
                year: Some(2020),
                duration_s: Some(200),
            }
        );
    }

    #[test]
    fn musique_depuis_le_chemin() {
        let r = recognize("Musique/Artiste Démo/Album Démo/01 - Premier.flac", None).unwrap();
        assert_eq!(
            r,
            Recognized::Track {
                artist: "Artiste Démo".into(),
                album: "Album Démo".into(),
                title: "Premier".into(),
                track: Some(1),
                year: None,
                duration_s: None,
            }
        );
        let r = recognize("Artiste - Titre.mp3", None).unwrap();
        assert!(
            matches!(r, Recognized::Track { ref artist, ref album, .. } if artist == "Artiste" && album == UNKNOWN_ALBUM)
        );
    }

    #[test]
    fn pas_un_media() {
        assert!(recognize("Film.srt", None).is_none());
        assert!(recognize("cover.jpg", None).is_none());
        assert!(recognize("._Film.mkv", None).is_none());
        assert_eq!(file_kind("FILM.MKV"), FileKind::Video);
    }
}
