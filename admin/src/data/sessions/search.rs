use entity::{sessions, QueryOrder, Uuid};
use entity::{sort::Sortable, users, Order, Select};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Search {
    pub with_expired: Option<bool>,
    pub user_id: Option<Uuid>,
    pub search: Option<String>,
    pub sort: Option<SessionsSort>,
    pub order: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl Validation for Search {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_in!(
            order,
            Into::<Vec<String>>::into(["asc".to_string(), "desc".to_string()])
        )]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![
            Modifier::new("search", |obj: &mut Self| {
                if let Some(s) = obj.search.as_deref() {
                    obj.search = Some(s.to_lowercase());
                }
            }),
            Modifier::new("order", |obj: &mut Self| {
                if let Some(s) = obj.order.as_deref() {
                    obj.order = Some(s.to_lowercase());
                }
            }),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum SessionsSort {
    Id,
    Email,
    #[default]
    CreatedAt,
    UpdatedAt,
    ExpiresAt,
}

impl Sortable for SessionsSort {
    type Entity = sessions::Entity;

    fn sort(&self, query: Select<Self::Entity>, order: Order) -> Select<Self::Entity> {
        match self {
            SessionsSort::Id => query.order_by(sessions::Column::Id, order),
            SessionsSort::Email => query.order_by(users::Column::Email, order),
            SessionsSort::CreatedAt => query.order_by(sessions::Column::CreatedAt, order),
            SessionsSort::UpdatedAt => query.order_by(sessions::Column::UpdatedAt, order),
            SessionsSort::ExpiresAt => query.order_by(sessions::Column::ExpiresAt, order),
        }
    }
}

impl Serialize for SessionsSort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            SessionsSort::Id => serializer.serialize_str("id"),
            SessionsSort::Email => serializer.serialize_str("email"),
            SessionsSort::CreatedAt => serializer.serialize_str("created_at"),
            SessionsSort::UpdatedAt => serializer.serialize_str("updated_at"),
            SessionsSort::ExpiresAt => serializer.serialize_str("expires_at"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for SessionsSort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SessionsSortVisitor;

        impl serde::de::Visitor<'_> for SessionsSortVisitor {
            type Value = SessionsSort;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a string representing a SessionsSort")
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<SessionsSort, E> {
                Ok(match s {
                    "id" => SessionsSort::Id,
                    "email" => SessionsSort::Email,
                    "created_at" => SessionsSort::CreatedAt,
                    "updated_at" => SessionsSort::UpdatedAt,
                    "expires_at" => SessionsSort::ExpiresAt,
                    _ => return Err(E::invalid_value(serde::de::Unexpected::Str(s), &self)),
                })
            }
        }

        deserializer.deserialize_any(SessionsSortVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::SessionsSort;
    use serde_json::json;

    #[derive(serde::Deserialize)]
    struct Test {
        _sort: SessionsSort,
    }

    #[test]
    fn test_serialize_enum_variants() {
        let variants = vec![
            "id".to_string(),
            "email".to_string(),
            "created_at".to_string(),
            "updated_at".to_string(),
            "expires_at".to_string(),
        ];

        for variant in variants {
            let sort = match variant.as_str() {
                "id" => SessionsSort::Id,
                "email" => SessionsSort::Email,
                "created_at" => SessionsSort::CreatedAt,
                "updated_at" => SessionsSort::UpdatedAt,
                "expires_at" => SessionsSort::ExpiresAt,
                _ => panic!("Unknown variant: {}", variant),
            };

            let _deserialize = serde_json::from_value::<Test>(json!({
                "_sort": variant,
            }))
            .unwrap();

            let serialized = serde_json::to_string(&sort).unwrap();

            assert_eq!(serialized, format!("\"{}\"", variant));
        }
    }

    #[test]
    #[should_panic]
    fn test_fail_with_bogus_data() {
        let _deserialize = serde_json::from_value::<Test>(json!({
            "_sort": "bogus",
        }))
        .unwrap();
    }

    #[test]
    fn test_serialize_search_struct() {
        let search = super::Search {
            with_expired: None,
            user_id: None,
            search: None,
            sort: Some(SessionsSort::Id),
            order: None,
            limit: None,
            offset: None,
        };

        let serialized = serde_json::to_string(&search).unwrap();

        assert_eq!(
            serialized,
            "{\"with_expired\":null,\"user_id\":null,\"search\":null,\"sort\":\"id\",\"order\":null,\"limit\":null,\"offset\":null}"
        );
    }

    #[test]
    fn test_deserialize_search_struct() {
        let search = super::Search {
            with_expired: None,
            user_id: None,
            search: None,
            sort: Some(SessionsSort::Id),
            order: None,
            limit: None,
            offset: None,
        };

        let deserialized = serde_json::from_str::<super::Search>(
            "{\"with_expired\":null,\"user_id\":null,\"search\":null,\"sort\":\"id\",\"order\":null,\"limit\":null,\"offset\":null}",
        )
        .unwrap();

        assert_eq!(deserialized, search);
    }
}
