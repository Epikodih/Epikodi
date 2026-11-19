//! Colonnes JSON ↔ structs typées. Une valeur illisible donne la valeur par
//! défaut (journalisée) plutôt qu'une erreur : la base ne doit pas rendre le
//! catalogue inutilisable pour un champ abîmé.

use serde::{de::DeserializeOwned, Serialize};

use epikodi_domain::{DomainError, DomainResult};

pub fn from_column<T: DeserializeOwned + Default>(text: &str, column: &str) -> T {
    serde_json::from_str(text).unwrap_or_else(|e| {
        tracing::warn!(column, "JSON illisible, valeurs par défaut : {e}");
        T::default()
    })
}

pub fn to_column<T: Serialize>(value: &T) -> DomainResult<String> {
    serde_json::to_string(value).map_err(DomainError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use epikodi_domain::profile::Preferences;

    #[test]
    fn json_abime_donne_les_valeurs_par_defaut() {
        let p: Preferences = from_column("{pas du json", "preferences");
        assert_eq!(p, Preferences::default());
    }

    #[test]
    fn aller_retour() {
        let p = Preferences {
            theme: "alternative".into(),
            ..Default::default()
        };
        let back: Preferences = from_column(&to_column(&p).unwrap(), "preferences");
        assert_eq!(back, p);
    }
}
