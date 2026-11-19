//! Catalogue (`media`, `media_details`).
//!
//! Le filtre parental est fait DANS la requête (sinon pagination et compteurs
//! sont faux). La classification effective d'un média est la sienne, sinon
//! celle de son parent, sinon celle de son grand-parent.

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, OptionalExtension, Row};

use epikodi_domain::media::{
    Media, MediaDetails, MediaKind, MediaPage, MediaPatch, MediaQuery, MediaSort,
};
use epikodi_domain::parental::Visibility;
use epikodi_domain::ports::MediaRepo;
use epikodi_domain::{DomainError, DomainResult, Id};

use super::{db_err, iso, parse_col, Store};
use crate::json;

pub(crate) const COLS: &str = "m.id, m.parent_id, m.kind, m.title, m.year, m.poster_path, \
    m.duration_s, m.sort_number, m.age_rating, m.match_status, m.source_id, m.uri, \
    m.parent_dir, m.size_bytes, m.modified_at, m.fingerprint, m.status, m.added_at";

/// `m` = le média, `p` = son parent, `g` = son grand-parent.
pub(crate) const FROM: &str = "FROM media m \
    LEFT JOIN media p ON p.id = m.parent_id \
    LEFT JOIN media g ON g.id = p.parent_id";

pub(crate) fn row(r: &Row) -> rusqlite::Result<Media> {
    Ok(Media {
        id: r.get(0)?,
        parent_id: r.get(1)?,
        kind: parse_col(2, r.get(2)?)?,
        title: r.get(3)?,
        year: r.get(4)?,
        poster_path: r.get(5)?,
        duration_s: r.get(6)?,
        sort_number: r.get(7)?,
        age_rating: r.get(8)?,
        match_status: parse_col(9, r.get(9)?)?,
        source_id: r.get(10)?,
        uri: r.get(11)?,
        parent_dir: r.get(12)?,
        size_bytes: r.get(13)?,
        modified_at: r.get(14)?,
        fingerprint: r.get(15)?,
        status: parse_col(16, r.get(16)?)?,
        added_at: iso(r.get(17)?),
    })
}

/// Condition SQL du filtre parental (l'âge est un `u8` : pas d'injection).
pub(crate) fn visible(vis: Visibility) -> String {
    match vis {
        Visibility::All => "1".into(),
        Visibility::MaxAge(age) => {
            format!("COALESCE(m.age_rating, p.age_rating, g.age_rating) <= {age}")
        }
    }
}

const PARENT_ORDER: &str = "m.sort_number IS NULL, m.sort_number, m.title COLLATE TITLE, m.id";
const TITLE_ORDER: &str = "m.title COLLATE TITLE, m.id";

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl Store {
    fn query_media(&self, sql: &str, args: &[Value]) -> DomainResult<Vec<Media>> {
        let mut st = self.conn.prepare(sql).map_err(db_err)?;
        let rows = st
            .query_map(params_from_iter(args.iter()), row)
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn collect_leaves(
        &self,
        m: Media,
        vis: Visibility,
        depth: u8,
        out: &mut Vec<Media>,
    ) -> DomainResult<()> {
        if m.kind.is_leaf() {
            out.push(m);
            return Ok(());
        }
        if depth >= 3 {
            return Ok(());
        }
        for child in self.children(m.id, vis)? {
            self.collect_leaves(child, vis, depth + 1, out)?;
        }
        Ok(())
    }
}

impl MediaRepo for Store {
    fn get_media(&self, id: Id) -> DomainResult<Option<Media>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM media m WHERE m.id = ?1"),
                [id],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn get_visible_media(&self, id: Id, vis: Visibility) -> DomainResult<Option<Media>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} {FROM} WHERE m.id = ?1 AND {}", visible(vis)),
                [id],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn list_media(&self, q: &MediaQuery, vis: Visibility) -> DomainResult<MediaPage> {
        let mut wh = vec![visible(vis)];
        let mut args: Vec<Value> = Vec::new();

        match q.parent_id {
            Some(pid) => {
                wh.push("m.parent_id = ?".into());
                args.push(Value::Integer(pid));
            }
            // Sans parent ni filtre : le niveau du haut (films, séries...).
            None if q.kind.is_none()
                && q.search.is_none()
                && q.genre.is_none()
                && q.year.is_none() =>
            {
                wh.push("m.parent_id IS NULL".into());
            }
            None => {}
        }
        if let Some(kind) = q.kind {
            wh.push("m.kind = ?".into());
            args.push(Value::Text(kind.as_str().into()));
        }
        if let Some(search) = q.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            wh.push("fold(m.title) LIKE ? ESCAPE '\\'".into());
            args.push(Value::Text(format!(
                "%{}%",
                escape_like(&crate::db::fold(search))
            )));
        }
        if let Some(genre) = &q.genre {
            wh.push(
                "EXISTS (SELECT 1 FROM media_details d, json_each(d.details, '$.genres') j \
                 WHERE d.media_id = m.id AND j.value = ? COLLATE NOCASE)"
                    .into(),
            );
            args.push(Value::Text(genre.clone()));
        }
        if let Some(year) = q.year {
            wh.push("m.year = ?".into());
            args.push(Value::Integer(year.into()));
        }
        let where_sql = wh.join(" AND ");

        let total: i64 = self
            .conn
            .query_row(
                &format!("SELECT COUNT(*) {FROM} WHERE {where_sql}"),
                params_from_iter(args.iter()),
                |r| r.get(0),
            )
            .map_err(db_err)?;

        let order = match q.sort {
            MediaSort::Default if q.parent_id.is_some() => PARENT_ORDER,
            MediaSort::Default | MediaSort::Title => TITLE_ORDER,
            MediaSort::Year => "m.year IS NULL, m.year DESC, m.title COLLATE TITLE, m.id",
            MediaSort::Added => "m.added_at DESC, m.id DESC",
        };
        let mut page_args = args.clone();
        page_args.push(Value::Integer(q.limit.into()));
        page_args.push(Value::Integer(q.offset.into()));
        let items = self.query_media(
            &format!("SELECT {COLS} {FROM} WHERE {where_sql} ORDER BY {order} LIMIT ? OFFSET ?"),
            &page_args,
        )?;
        Ok(MediaPage {
            items,
            total: total as u64,
        })
    }

    fn children(&self, parent_id: Id, vis: Visibility) -> DomainResult<Vec<Media>> {
        self.query_media(
            &format!(
                "SELECT {COLS} {FROM} WHERE m.parent_id = ? AND {} ORDER BY {PARENT_ORDER}",
                visible(vis)
            ),
            &[Value::Integer(parent_id)],
        )
    }

    fn leaves_under(&self, id: Id, vis: Visibility) -> DomainResult<Vec<Media>> {
        let Some(m) = self.get_visible_media(id, vis)? else {
            return Err(DomainError::NotFound { entity: "média" });
        };
        let mut out = Vec::new();
        self.collect_leaves(m, vis, 0, &mut out)?;
        Ok(out)
    }

    fn recently_added(&self, vis: Visibility, limit: u32) -> DomainResult<Vec<Media>> {
        self.query_media(
            &format!(
                "SELECT {COLS} {FROM} WHERE m.parent_id IS NULL AND m.status = 'available' AND {} \
                 ORDER BY m.added_at DESC, m.id DESC LIMIT ?",
                visible(vis)
            ),
            &[Value::Integer(limit.into())],
        )
    }

    fn genres(&self, vis: Visibility) -> DomainResult<Vec<(String, u64)>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT j.value, COUNT(*) {FROM} JOIN media_details d ON d.media_id = m.id, \
                 json_each(d.details, '$.genres') j \
                 WHERE m.parent_id IS NULL AND {} GROUP BY j.value ORDER BY j.value COLLATE TITLE",
                visible(vis)
            ))
            .map_err(db_err)?;
        let rows = st
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn years(&self, vis: Visibility) -> DomainResult<Vec<(i32, u64)>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT m.year, COUNT(*) {FROM} WHERE m.year IS NOT NULL AND m.parent_id IS NULL \
                 AND {} GROUP BY m.year ORDER BY m.year DESC",
                visible(vis)
            ))
            .map_err(db_err)?;
        let rows = st
            .query_map([], |r| {
                Ok((r.get::<_, i32>(0)?, r.get::<_, i64>(1)? as u64))
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn get_details(&self, id: Id) -> DomainResult<Option<MediaDetails>> {
        let text: Option<String> = self
            .conn
            .query_row(
                "SELECT details FROM media_details WHERE media_id = ?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_err)?;
        Ok(text.map(|t| json::from_column(&t, "media_details.details")))
    }

    fn set_details(&self, id: Id, details: &MediaDetails) -> DomainResult<()> {
        self.conn
            .execute(
                "INSERT INTO media_details (media_id, details) VALUES (?1, ?2)
                 ON CONFLICT (media_id) DO UPDATE SET details = excluded.details",
                params![id, json::to_column(details)?],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn patch_media(&self, id: Id, patch: &MediaPatch) -> DomainResult<()> {
        let mut sets: Vec<&str> = Vec::new();
        let mut args: Vec<Value> = Vec::new();
        let opt_int = |v: Option<i64>| v.map_or(Value::Null, Value::Integer);
        if let Some(t) = &patch.title {
            sets.push("title = ?");
            args.push(Value::Text(t.clone()));
        }
        if let Some(y) = patch.year {
            sets.push("year = ?");
            args.push(opt_int(y.map(i64::from)));
        }
        if let Some(a) = patch.age_rating {
            sets.push("age_rating = ?");
            args.push(opt_int(a.map(i64::from)));
        }
        if let Some(n) = patch.sort_number {
            sets.push("sort_number = ?");
            args.push(opt_int(n));
        }
        if let Some(d) = patch.duration_s {
            sets.push("duration_s = ?");
            args.push(opt_int(d));
        }
        if let Some(p) = &patch.poster_path {
            sets.push("poster_path = ?");
            args.push(p.clone().map_or(Value::Null, Value::Text));
        }
        if let Some(s) = patch.match_status {
            sets.push("match_status = ?");
            args.push(Value::Text(s.as_str().into()));
        }
        if sets.is_empty() {
            return Ok(());
        }
        args.push(Value::Integer(id));
        let n = self
            .conn
            .execute(
                &format!("UPDATE media SET {} WHERE id = ?", sets.join(", ")),
                params_from_iter(args.iter()),
            )
            .map_err(db_err)?;
        if n == 0 {
            return Err(DomainError::NotFound { entity: "média" });
        }
        Ok(())
    }
}

/// Insère un média de démonstration (tests des autres modules et de l'API).
#[doc(hidden)]
pub fn insert_test_media(
    store: &Store,
    parent_id: Option<Id>,
    kind: MediaKind,
    title: &str,
    age_rating: Option<u8>,
) -> Id {
    let uri = kind
        .is_leaf()
        .then(|| format!("file:///demo/{title}-{}", rand_suffix(store)));
    store
        .conn
        .execute(
            "INSERT INTO media (parent_id, kind, title, age_rating, match_status, uri, added_at)
             VALUES (?1, ?2, ?3, ?4, 'auto', ?5, ?6)",
            params![
                parent_id,
                kind.as_str(),
                title,
                age_rating,
                uri,
                crate::now_iso()
            ],
        )
        .unwrap();
    store.conn.last_insert_rowid()
}

fn rand_suffix(store: &Store) -> i64 {
    store
        .conn
        .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM media", [], |r| {
            r.get(0)
        })
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use epikodi_domain::media::{MatchStatus, MediaKind::*};

    fn q() -> MediaQuery {
        MediaQuery {
            limit: 50,
            ..Default::default()
        }
    }

    fn demo() -> (Store, Id, Id) {
        let s = Store::in_memory();
        insert_test_media(&s, None, Movie, "Sintel", Some(10));
        insert_test_media(&s, None, Movie, "Big Buck Bunny", Some(0));
        insert_test_media(&s, None, Movie, "Film 16 ans", Some(16));
        insert_test_media(&s, None, Movie, "mon_film_2019_final", None);
        let series = insert_test_media(&s, None, Series, "Série Démo", Some(10));
        let s1 = insert_test_media(&s, Some(series), Season, "Saison 1", None);
        let e1 = insert_test_media(&s, Some(s1), Episode, "Épisode 1", None);
        insert_test_media(&s, Some(s1), Episode, "Épisode 2", Some(16));
        s.conn
            .execute("UPDATE media SET sort_number = 1 WHERE id = ?1", [e1])
            .unwrap();
        (s, series, s1)
    }

    fn titles(p: &MediaPage) -> Vec<&str> {
        p.items.iter().map(|m| m.title.as_str()).collect()
    }

    #[test]
    fn niveau_du_haut_trie_par_titre() {
        let (s, ..) = demo();
        let page = s.list_media(&q(), Visibility::All).unwrap();
        assert_eq!(
            titles(&page),
            [
                "Big Buck Bunny",
                "Film 16 ans",
                "mon_film_2019_final",
                "Série Démo",
                "Sintel"
            ]
        );
        assert_eq!(page.total, 5);
    }

    #[test]
    fn filtre_parental_dans_la_requete() {
        let (s, ..) = demo();
        let page = s.list_media(&q(), Visibility::MaxAge(10)).unwrap();
        assert_eq!(titles(&page), ["Big Buck Bunny", "Série Démo", "Sintel"]);
        assert_eq!(page.total, 3, "le total compte seulement les visibles");
        let guest = s.list_media(&q(), Visibility::MaxAge(0)).unwrap();
        assert_eq!(titles(&guest), ["Big Buck Bunny"]);
    }

    #[test]
    fn heritage_de_la_classification() {
        let (s, _, season) = demo();
        let mut query = q();
        query.parent_id = Some(season);
        // Épisode 1 sans note hérite des 10 ans de la série ; Épisode 2 a 16.
        let kid = s.list_media(&query, Visibility::MaxAge(12)).unwrap();
        assert_eq!(titles(&kid), ["Épisode 1"]);
        let adult = s.list_media(&query, Visibility::All).unwrap();
        assert_eq!(
            titles(&adult),
            ["Épisode 1", "Épisode 2"],
            "tri par sort_number"
        );
    }

    #[test]
    fn pagination() {
        let (s, ..) = demo();
        let mut query = q();
        query.limit = 2;
        query.offset = 1;
        let page = s.list_media(&query, Visibility::All).unwrap();
        assert_eq!(titles(&page), ["Film 16 ans", "mon_film_2019_final"]);
        assert_eq!(page.total, 5);
    }

    #[test]
    fn recherche_et_type() {
        let (s, ..) = demo();
        let mut query = q();
        query.search = Some("épisode".into());
        assert_eq!(
            s.list_media(&query, Visibility::All).unwrap().total,
            2,
            "sans accents"
        );
        query.search = Some("EPISODE".into());
        assert_eq!(s.list_media(&query, Visibility::All).unwrap().total, 2);
        query.search = Some("100%".into());
        assert_eq!(
            s.list_media(&query, Visibility::All).unwrap().total,
            0,
            "% échappé"
        );
        let mut query = q();
        query.kind = Some(Episode);
        assert_eq!(s.list_media(&query, Visibility::All).unwrap().total, 2);
    }

    #[test]
    fn feuilles_sous_une_serie() {
        let (s, series, _) = demo();
        let leaves = s.leaves_under(series, Visibility::All).unwrap();
        assert_eq!(leaves.len(), 2);
        let kid = s.leaves_under(series, Visibility::MaxAge(12)).unwrap();
        assert_eq!(kid.len(), 1);
    }

    #[test]
    fn media_masque_introuvable_pour_un_restreint() {
        let (s, ..) = demo();
        let film16 = s
            .list_media(&q(), Visibility::All)
            .unwrap()
            .items
            .into_iter()
            .find(|m| m.title == "Film 16 ans")
            .unwrap();
        assert!(s
            .get_visible_media(film16.id, Visibility::MaxAge(12))
            .unwrap()
            .is_none());
        assert!(s
            .get_visible_media(film16.id, Visibility::All)
            .unwrap()
            .is_some());
    }

    #[test]
    fn details_genres_et_correction() {
        let (s, series, _) = demo();
        let d = MediaDetails {
            genres: vec!["Animation".into(), "Drame".into()],
            ..Default::default()
        };
        s.set_details(series, &d).unwrap();
        assert_eq!(s.get_details(series).unwrap().unwrap(), d);
        assert_eq!(
            s.genres(Visibility::All).unwrap(),
            [("Animation".into(), 1), ("Drame".into(), 1)]
        );
        let mut query = q();
        query.genre = Some("drame".into());
        assert_eq!(
            titles(&s.list_media(&query, Visibility::All).unwrap()),
            ["Série Démo"]
        );

        s.patch_media(
            series,
            &MediaPatch {
                title: Some("Autre titre".into()),
                year: Some(Some(2021)),
                match_status: Some(MatchStatus::Manual),
                ..Default::default()
            },
        )
        .unwrap();
        let m = s.get_media(series).unwrap().unwrap();
        assert_eq!(m.title, "Autre titre");
        assert_eq!(m.year, Some(2021));
        assert_eq!(m.match_status, MatchStatus::Manual);
        assert_eq!(s.years(Visibility::All).unwrap(), [(2021, 1)]);
    }

    #[test]
    fn supprimer_une_serie_supprime_saisons_et_episodes() {
        let (s, series, _) = demo();
        s.conn
            .execute("DELETE FROM media WHERE id = ?1", [series])
            .unwrap();
        let mut query = q();
        query.kind = Some(Episode);
        assert_eq!(s.list_media(&query, Visibility::All).unwrap().total, 0);
    }
}
