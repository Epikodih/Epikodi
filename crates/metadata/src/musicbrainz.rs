//! MusicBrainz : artistes et albums (genres, date de sortie) et pochettes du
//! Cover Art Archive. Sans clé, mais limité à 1 requête par seconde et un
//! User-Agent identifiant l'application est obligatoire.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;

use epikodi_domain::media::{MediaDetails, MediaKind};
use epikodi_domain::ports::{
    MetadataCandidate, MetadataError, MetadataProvider, MetadataQuery, MetadataRecord,
};

use crate::provider::{encode, str_field, year_of, Http};

pub const NAME: &str = "musicbrainz";
const API: &str = "https://musicbrainz.org/ws/2";
const COVERS: &str = "https://coverartarchive.org";
const MIN_INTERVAL: Duration = Duration::from_millis(1100);

pub struct MusicBrainz {
    http: Arc<dyn Http>,
    last_call: Mutex<Option<Instant>>,
    throttle: bool,
}

impl MusicBrainz {
    pub fn new(http: Arc<dyn Http>) -> Self {
        Self {
            http,
            last_call: Mutex::new(None),
            throttle: true,
        }
    }

    fn get(&self, path_and_query: &str) -> Result<Value, MetadataError> {
        if self.throttle {
            let mut last = self.last_call.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(t) = *last {
                let elapsed = t.elapsed();
                if elapsed < MIN_INTERVAL {
                    std::thread::sleep(MIN_INTERVAL - elapsed);
                }
            }
            *last = Some(Instant::now());
        }
        self.http.get_json(&format!("{API}{path_and_query}"), &[])
    }

    fn genres(v: &Value) -> Vec<String> {
        v["genres"]
            .as_array()
            .map(|g| {
                g.iter()
                    .filter_map(|x| str_field(x, "name"))
                    .take(8)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Échappe une valeur pour la syntaxe de recherche Lucene de MusicBrainz.
fn lucene(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn is_mbid(s: &str) -> bool {
    s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

impl MetadataProvider for MusicBrainz {
    fn name(&self) -> &str {
        NAME
    }

    fn supports(&self, kind: MediaKind) -> bool {
        matches!(kind, MediaKind::Artist | MediaKind::Album)
    }

    fn search(&self, q: &MetadataQuery) -> Result<Vec<MetadataCandidate>, MetadataError> {
        match q.kind {
            Some(MediaKind::Artist) => {
                let query = format!("artist:\"{}\"", lucene(&q.title));
                let v = self.get(&format!(
                    "/artist?fmt=json&limit=5&query={}",
                    encode(&query)
                ))?;
                Ok(v["artists"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| {
                                Some(MetadataCandidate {
                                    provider: NAME.into(),
                                    external_id: str_field(x, "id")?,
                                    kind: MediaKind::Artist,
                                    title: str_field(x, "name")?,
                                    year: year_of(x["life-span"]["begin"].as_str()),
                                    overview: str_field(x, "disambiguation"),
                                    poster_url: None,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default())
            }
            Some(MediaKind::Album) => {
                let mut query = format!("releasegroup:\"{}\"", lucene(&q.title));
                if let Some(a) = &q.artist {
                    query.push_str(&format!(" AND artist:\"{}\"", lucene(a)));
                }
                let v = self.get(&format!(
                    "/release-group?fmt=json&limit=5&query={}",
                    encode(&query)
                ))?;
                Ok(v["release-groups"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| {
                                let id = str_field(x, "id")?;
                                Some(MetadataCandidate {
                                    provider: NAME.into(),
                                    poster_url: Some(format!(
                                        "{COVERS}/release-group/{id}/front-250"
                                    )),
                                    external_id: id,
                                    kind: MediaKind::Album,
                                    title: str_field(x, "title")?,
                                    year: year_of(x["first-release-date"].as_str()),
                                    overview: x["artist-credit"][0]["name"]
                                        .as_str()
                                        .map(String::from),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default())
            }
            _ => Ok(Vec::new()),
        }
    }

    fn fetch(&self, kind: MediaKind, external_id: &str) -> Result<MetadataRecord, MetadataError> {
        if !is_mbid(external_id) {
            return Err(MetadataError::NotFound);
        }
        let (v, poster_url) = match kind {
            MediaKind::Artist => (
                self.get(&format!("/artist/{external_id}?fmt=json&inc=genres"))?,
                None,
            ),
            MediaKind::Album => (
                self.get(&format!(
                    "/release-group/{external_id}?fmt=json&inc=genres+artist-credits"
                ))?,
                Some(format!("{COVERS}/release-group/{external_id}/front-500")),
            ),
            _ => return Err(MetadataError::NotFound),
        };
        let date = v["first-release-date"]
            .as_str()
            .or(v["life-span"]["begin"].as_str());
        Ok(MetadataRecord {
            title: str_field(&v, "title").or_else(|| str_field(&v, "name")),
            year: year_of(date),
            // La musique est visible des profils restreints une fois reconnue.
            age_rating: Some(0),
            duration_s: None,
            poster_url,
            details: MediaDetails {
                overview: str_field(&v, "disambiguation"),
                release_date: date.map(String::from),
                genres: Self::genres(&v),
                external_ids: [("musicbrainz".to_string(), Value::String(external_id.into()))]
                    .into_iter()
                    .collect(),
                ..Default::default()
            },
        })
    }

    fn fetch_image(&self, url: &str) -> Result<Vec<u8>, MetadataError> {
        if !url.starts_with(COVERS) {
            return Err(MetadataError::Invalid(
                "image hors du Cover Art Archive".into(),
            ));
        }
        self.http.get_bytes(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::fake::FakeHttp;
    use serde_json::json;

    const ID: &str = "0f0e1b5b-0000-4000-8000-000000000001";

    fn mb(h: FakeHttp) -> (MusicBrainz, Arc<FakeHttp>) {
        let http = Arc::new(h);
        let mut m = MusicBrainz::new(http.clone());
        m.throttle = false;
        (m, http)
    }

    #[test]
    fn recherche_d_album_avec_artiste() {
        let mut h = FakeHttp::default();
        h.routes.insert(
            "/release-group?",
            json!({"release-groups": [{"id": ID, "title": "Album Démo", "first-release-date": "2019-03-01",
                "artist-credit": [{"name": "Artiste Démo"}]}]}),
        );
        let (m, http) = mb(h);
        let r = m
            .search(&MetadataQuery {
                kind: Some(MediaKind::Album),
                title: "Album \"Démo\"".into(),
                artist: Some("Artiste Démo".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r[0].external_id, ID);
        assert_eq!(r[0].year, Some(2019));
        let url = &http.calls.lock().unwrap()[0];
        assert!(url.contains("artist%3A%22Artiste%20D%C3%A9mo%22"));
        assert!(url.contains("%5C%22D%C3%A9mo%5C%22"), "guillemets échappés");
    }

    #[test]
    fn fiche_d_album() {
        let mut h = FakeHttp::default();
        h.routes.insert(
            "/release-group/",
            json!({"id": ID, "title": "Album Démo", "first-release-date": "2019",
                "genres": [{"name": "electronic"}]}),
        );
        let (m, _) = mb(h);
        let r = m.fetch(MediaKind::Album, ID).unwrap();
        assert_eq!(r.title.as_deref(), Some("Album Démo"));
        assert_eq!(r.year, Some(2019));
        assert_eq!(r.details.genres, ["electronic"]);
        assert!(r.poster_url.unwrap().starts_with(COVERS));
        assert!(m.fetch(MediaKind::Album, "pas-un-id").is_err());
    }

    #[test]
    fn films_non_pris_en_charge() {
        let (m, _) = mb(FakeHttp::default());
        assert!(!m.supports(MediaKind::Movie));
        assert!(m.supports(MediaKind::Album));
    }
}
