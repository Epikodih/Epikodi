//! TheMovieDB : films et séries (affiche, fond, résumé, casting, genres,
//! durée, note, classification française).
//!
//! Clé : variable d'environnement `TMDB_API_KEY` (clé v3 ou jeton v4).

use std::sync::Arc;

use serde_json::Value;

use epikodi_domain::media::{CastMember, MediaDetails, MediaKind};
use epikodi_domain::ports::{
    MetadataCandidate, MetadataError, MetadataProvider, MetadataQuery, MetadataRecord,
};

use crate::provider::{encode, str_field, year_of, Http};

pub const NAME: &str = "tmdb";
const API: &str = "https://api.themoviedb.org/3";
const IMAGES: &str = "https://image.tmdb.org/t/p";

pub struct Tmdb {
    http: Arc<dyn Http>,
    api_key: String,
    language: String,
}

impl Tmdb {
    pub fn new(
        http: Arc<dyn Http>,
        api_key: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            http,
            api_key: api_key.into(),
            language: language.into(),
        }
    }

    /// Clé v3 (32 caractères) en paramètre, jeton v4 (long) en en-tête.
    fn get(&self, path: &str, params: &str) -> Result<Value, MetadataError> {
        let lang = encode(&self.language);
        if self.api_key.len() > 40 {
            let auth = format!("Bearer {}", self.api_key);
            self.http.get_json(
                &format!("{API}{path}?language={lang}{params}"),
                &[("Authorization", &auth)],
            )
        } else {
            self.http.get_json(
                &format!(
                    "{API}{path}?api_key={}&language={lang}{params}",
                    encode(&self.api_key)
                ),
                &[],
            )
        }
    }

    fn image(path: Option<String>, size: &str) -> Option<String> {
        path.map(|p| format!("{IMAGES}/{size}{p}"))
    }

    fn details(v: &Value) -> MediaDetails {
        MediaDetails {
            overview: str_field(v, "overview"),
            original_title: str_field(v, "original_title")
                .or_else(|| str_field(v, "original_name")),
            tagline: str_field(v, "tagline"),
            release_date: str_field(v, "release_date").or_else(|| str_field(v, "first_air_date")),
            backdrop: Self::image(str_field(v, "backdrop_path"), "w1280"),
            rating: v
                .get("vote_average")
                .and_then(Value::as_f64)
                .filter(|r| *r > 0.0),
            genres: v["genres"]
                .as_array()
                .map(|g| g.iter().filter_map(|x| str_field(x, "name")).collect())
                .unwrap_or_default(),
            cast: v["credits"]["cast"]
                .as_array()
                .map(|c| {
                    c.iter()
                        .take(12)
                        .filter_map(|p| {
                            Some(CastMember {
                                name: str_field(p, "name")?,
                                role: Some("acteur".into()),
                                character: str_field(p, "character"),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            external_ids: [("tmdb".to_string(), v["id"].clone())]
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }
}

/// Classification (France en priorité, sinon États-Unis) → âge. On arrondit
/// toujours vers le haut : 13 ans devient 16, 17 devient 18.
pub fn certification_to_age(cert: &str) -> Option<u8> {
    let c = cert.trim().trim_start_matches('-').to_ascii_uppercase();
    match c.as_str() {
        "" | "NR" | "UNRATED" => None,
        "U" | "TP" | "0" | "TOUS PUBLICS" | "G" | "TV-Y" | "TV-G" => Some(0),
        "10" | "PG" | "TV-Y7" | "TV-PG" => Some(10),
        "12" => Some(12),
        "16" | "PG-13" | "TV-14" => Some(16),
        "18" | "R" | "NC-17" | "TV-MA" => Some(18),
        _ => None,
    }
}

fn movie_certification(v: &Value) -> Option<u8> {
    let results = v["release_dates"]["results"].as_array()?;
    ["FR", "US"].iter().find_map(|country| {
        results.iter().find(|r| r["iso_3166_1"] == *country)?["release_dates"]
            .as_array()?
            .iter()
            .find_map(|d| d["certification"].as_str().and_then(certification_to_age))
    })
}

fn tv_certification(v: &Value) -> Option<u8> {
    let results = v["content_ratings"]["results"].as_array()?;
    ["FR", "US"].iter().find_map(|country| {
        results.iter().find(|r| r["iso_3166_1"] == *country)?["rating"]
            .as_str()
            .and_then(certification_to_age)
    })
}

impl MetadataProvider for Tmdb {
    fn name(&self) -> &str {
        NAME
    }

    fn supports(&self, kind: MediaKind) -> bool {
        matches!(kind, MediaKind::Movie | MediaKind::Series)
    }

    fn search(&self, q: &MetadataQuery) -> Result<Vec<MetadataCandidate>, MetadataError> {
        let kind = q.kind.unwrap_or(MediaKind::Movie);
        let (path, year_param, title_key, date_key) = match kind {
            MediaKind::Series => (
                "/search/tv",
                "first_air_date_year",
                "name",
                "first_air_date",
            ),
            _ => ("/search/movie", "year", "title", "release_date"),
        };
        let mut params = format!("&include_adult=false&query={}", encode(&q.title));
        if let Some(y) = q.year {
            params.push_str(&format!("&{year_param}={y}"));
        }
        let v = self.get(path, &params)?;
        Ok(v["results"]
            .as_array()
            .map(|r| {
                r.iter()
                    .take(10)
                    .filter_map(|x| {
                        Some(MetadataCandidate {
                            provider: NAME.into(),
                            external_id: x["id"].as_i64()?.to_string(),
                            kind,
                            title: str_field(x, title_key)?,
                            year: year_of(x[date_key].as_str()),
                            overview: str_field(x, "overview"),
                            poster_url: Self::image(str_field(x, "poster_path"), "w342"),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    fn fetch(&self, kind: MediaKind, external_id: &str) -> Result<MetadataRecord, MetadataError> {
        if !external_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(MetadataError::NotFound);
        }
        match kind {
            MediaKind::Series => {
                let v = self.get(
                    &format!("/tv/{external_id}"),
                    "&append_to_response=credits,content_ratings",
                )?;
                Ok(MetadataRecord {
                    title: str_field(&v, "name"),
                    year: year_of(v["first_air_date"].as_str()),
                    age_rating: tv_certification(&v),
                    duration_s: None,
                    poster_url: Self::image(str_field(&v, "poster_path"), "w500"),
                    details: Self::details(&v),
                })
            }
            MediaKind::Movie => {
                let v = self.get(
                    &format!("/movie/{external_id}"),
                    "&append_to_response=credits,release_dates",
                )?;
                Ok(MetadataRecord {
                    title: str_field(&v, "title"),
                    year: year_of(v["release_date"].as_str()),
                    age_rating: movie_certification(&v),
                    duration_s: v["runtime"].as_i64().filter(|r| *r > 0).map(|m| m * 60),
                    poster_url: Self::image(str_field(&v, "poster_path"), "w500"),
                    details: Self::details(&v),
                })
            }
            _ => Err(MetadataError::NotFound),
        }
    }

    fn fetch_image(&self, url: &str) -> Result<Vec<u8>, MetadataError> {
        if !url.starts_with(IMAGES) {
            return Err(MetadataError::Invalid("image hors de TMDB".into()));
        }
        self.http.get_bytes(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::fake::FakeHttp;
    use serde_json::json;

    fn tmdb(http: FakeHttp) -> (Tmdb, Arc<FakeHttp>) {
        let http = Arc::new(http);
        (Tmdb::new(http.clone(), "cle-de-test", "fr-FR"), http)
    }

    #[test]
    fn recherche_de_film() {
        let mut h = FakeHttp::default();
        h.routes.insert(
            "/search/movie",
            json!({"results": [{"id": 45745, "title": "Sintel", "release_date": "2010-09-27",
                "overview": "Une jeune fille...", "poster_path": "/p.jpg"}]}),
        );
        let (t, http) = tmdb(h);
        let r = t
            .search(&MetadataQuery {
                kind: Some(MediaKind::Movie),
                title: "Sintel".into(),
                year: Some(2010),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].external_id, "45745");
        assert_eq!(r[0].year, Some(2010));
        assert_eq!(
            r[0].poster_url.as_deref(),
            Some("https://image.tmdb.org/t/p/w342/p.jpg")
        );
        let url = &http.calls.lock().unwrap()[0];
        assert!(
            url.contains("query=Sintel")
                && url.contains("year=2010")
                && url.contains("language=fr-FR")
        );
    }

    #[test]
    fn fiche_de_film_avec_classification_francaise() {
        let mut h = FakeHttp::default();
        h.routes.insert(
            "/movie/45745",
            json!({
                "id": 45745, "title": "Sintel", "release_date": "2010-09-27", "runtime": 15,
                "overview": "Résumé", "vote_average": 7.2, "poster_path": "/p.jpg",
                "genres": [{"name": "Animation"}, {"name": "Fantastique"}],
                "credits": {"cast": [{"name": "Halina Reijn", "character": "Sintel"}]},
                "release_dates": {"results": [
                    {"iso_3166_1": "US", "release_dates": [{"certification": "PG-13"}]},
                    {"iso_3166_1": "FR", "release_dates": [{"certification": ""}, {"certification": "10"}]}
                ]}
            }),
        );
        let (t, _) = tmdb(h);
        let r = t.fetch(MediaKind::Movie, "45745").unwrap();
        assert_eq!(r.title.as_deref(), Some("Sintel"));
        assert_eq!(r.duration_s, Some(900));
        assert_eq!(r.age_rating, Some(10), "la France prime");
        assert_eq!(r.details.genres, ["Animation", "Fantastique"]);
        assert_eq!(r.details.cast[0].character.as_deref(), Some("Sintel"));
        assert_eq!(r.details.external_ids["tmdb"], 45745);
    }

    #[test]
    fn classifications() {
        assert_eq!(certification_to_age("U"), Some(0));
        assert_eq!(certification_to_age("-12"), Some(12));
        assert_eq!(
            certification_to_age("PG-13"),
            Some(16),
            "arrondi vers le haut"
        );
        assert_eq!(certification_to_age("TV-MA"), Some(18));
        assert_eq!(certification_to_age(""), None);
    }

    #[test]
    fn hors_connexion() {
        let (t, _) = tmdb(FakeHttp {
            offline: true,
            ..Default::default()
        });
        let q = MetadataQuery {
            title: "x".into(),
            ..Default::default()
        };
        assert!(matches!(t.search(&q), Err(MetadataError::Unavailable(_))));
    }

    #[test]
    fn identifiant_invalide_refuse_sans_appel() {
        let (t, http) = tmdb(FakeHttp::default());
        assert!(t.fetch(MediaKind::Movie, "1/../../x").is_err());
        assert!(http.calls.lock().unwrap().is_empty());
    }
}
