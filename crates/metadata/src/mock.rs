//! Catalogue démo intégré : les films libres de la Blender Foundation
//! (CC BY). Sert hors connexion, pour la démonstration et dans les tests.

use epikodi_domain::media::{MediaDetails, MediaKind};
use epikodi_domain::ports::{
    MetadataCandidate, MetadataError, MetadataProvider, MetadataQuery, MetadataRecord,
};

pub const NAME: &str = "demo";

struct Entry {
    id: &'static str,
    title: &'static str,
    year: i32,
    minutes: i64,
    age: u8,
    genres: &'static [&'static str],
    overview: &'static str,
}

const CATALOG: &[Entry] = &[
    Entry {
        id: "big-buck-bunny",
        title: "Big Buck Bunny",
        year: 2008,
        minutes: 10,
        age: 0,
        genres: &["Animation", "Comédie"],
        overview: "Un gros lapin au grand cœur se venge de trois rongeurs farceurs.",
    },
    Entry {
        id: "elephants-dream",
        title: "Elephants Dream",
        year: 2006,
        minutes: 11,
        age: 10,
        genres: &["Animation", "Science-fiction"],
        overview: "Deux hommes explorent une machine étrange et infinie.",
    },
    Entry {
        id: "sintel",
        title: "Sintel",
        year: 2010,
        minutes: 15,
        age: 10,
        genres: &["Animation", "Fantastique"],
        overview: "Une jeune femme part à la recherche de son bébé dragon.",
    },
    Entry {
        id: "tears-of-steel",
        title: "Tears of Steel",
        year: 2012,
        minutes: 12,
        age: 12,
        genres: &["Science-fiction", "Action"],
        overview: "Des scientifiques tentent de sauver le monde des robots à Amsterdam.",
    },
    Entry {
        id: "cosmos-laundromat",
        title: "Cosmos Laundromat",
        year: 2015,
        minutes: 12,
        age: 10,
        genres: &["Animation", "Fantastique"],
        overview: "Un mouton suicidaire rencontre un vendeur mystérieux.",
    },
    Entry {
        id: "caminandes-llamigos",
        title: "Caminandes 3: Llamigos",
        year: 2016,
        minutes: 3,
        age: 0,
        genres: &["Animation", "Comédie"],
        overview: "Koro le lama affronte un manchot gourmand.",
    },
    Entry {
        id: "agent-327",
        title: "Agent 327: Operation Barbershop",
        year: 2017,
        minutes: 4,
        age: 10,
        genres: &["Animation", "Action"],
        overview: "Un agent secret néerlandais enquête dans un salon de coiffure.",
    },
    Entry {
        id: "spring",
        title: "Spring",
        year: 2019,
        minutes: 8,
        age: 0,
        genres: &["Animation", "Fantastique"],
        overview: "Une bergère et son chien affrontent les esprits de la nature.",
    },
    Entry {
        id: "coffee-run",
        title: "Coffee Run",
        year: 2020,
        minutes: 3,
        age: 0,
        genres: &["Animation"],
        overview: "Une course effrénée pour un café.",
    },
    Entry {
        id: "sprite-fright",
        title: "Sprite Fright",
        year: 2021,
        minutes: 10,
        age: 12,
        genres: &["Animation", "Horreur", "Comédie"],
        overview: "Des adolescents rencontrent des lutins pas si mignons.",
    },
    Entry {
        id: "charge",
        title: "Charge",
        year: 2022,
        minutes: 4,
        age: 12,
        genres: &["Animation", "Action"],
        overview: "Dans un futur sombre, un vieil homme cherche de l'énergie.",
    },
    Entry {
        id: "wing-it",
        title: "Wing It!",
        year: 2023,
        minutes: 4,
        age: 0,
        genres: &["Animation", "Comédie"],
        overview: "Un chat et un chien construisent une fusée.",
    },
];

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[derive(Default)]
pub struct DemoCatalog;

impl DemoCatalog {
    fn find(&self, q: &MetadataQuery) -> impl Iterator<Item = &'static Entry> {
        let wanted = normalize(&q.title);
        CATALOG.iter().filter(move |e| {
            let t = normalize(e.title);
            !wanted.is_empty() && (t.contains(&wanted) || wanted.contains(&t))
        })
    }
}

impl MetadataProvider for DemoCatalog {
    fn name(&self) -> &str {
        NAME
    }

    fn supports(&self, kind: MediaKind) -> bool {
        kind == MediaKind::Movie
    }

    fn search(&self, q: &MetadataQuery) -> Result<Vec<MetadataCandidate>, MetadataError> {
        Ok(self
            .find(q)
            .map(|e| MetadataCandidate {
                provider: NAME.into(),
                external_id: e.id.into(),
                kind: MediaKind::Movie,
                title: e.title.into(),
                year: Some(e.year),
                overview: Some(e.overview.into()),
                poster_url: None,
            })
            .collect())
    }

    fn fetch(&self, _kind: MediaKind, external_id: &str) -> Result<MetadataRecord, MetadataError> {
        let e = CATALOG
            .iter()
            .find(|e| e.id == external_id)
            .ok_or(MetadataError::NotFound)?;
        Ok(MetadataRecord {
            title: Some(e.title.into()),
            year: Some(e.year),
            age_rating: Some(e.age),
            duration_s: Some(e.minutes * 60),
            poster_url: None,
            details: MediaDetails {
                overview: Some(e.overview.into()),
                genres: e.genres.iter().map(|g| g.to_string()).collect(),
                extra: [(
                    "license".to_string(),
                    serde_json::Value::String("CC BY — Blender Foundation".into()),
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            },
        })
    }

    fn fetch_image(&self, _url: &str) -> Result<Vec<u8>, MetadataError> {
        Err(MetadataError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrouve_les_films_libres() {
        let c = DemoCatalog;
        let r = c
            .search(&MetadataQuery {
                title: "big buck bunny".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r[0].external_id, "big-buck-bunny");
        let f = c.fetch(MediaKind::Movie, "sintel").unwrap();
        assert_eq!(f.age_rating, Some(10));
        assert_eq!(f.duration_s, Some(900));
        assert!(c.fetch(MediaKind::Movie, "inconnu").is_err());
        assert!(c.search(&MetadataQuery::default()).unwrap().is_empty());
    }
}
