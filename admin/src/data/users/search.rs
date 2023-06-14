use entity::QueryOrder;
use entity::{sort::Sortable, users, Order, Select};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Search {
    pub search: Option<String>,
    pub sort: Option<UsersSort>,
    pub order: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl Validation for Search {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![rule_in!(
            order,
            vec!["asc".to_string(), "desc".to_string(),]
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

#[derive(Debug, Clone, PartialEq)]
pub enum UsersSort {
    Id,
    Email,
    CreatedAt,
    UpdatedAt,
}

impl Sortable for UsersSort {
    type Entity = users::Entity;

    fn sort(&self, query: Select<Self::Entity>, order: Order) -> Select<Self::Entity> {
        match self {
            UsersSort::Id => query.order_by(users::Column::Id, order),
            UsersSort::Email => query.order_by(users::Column::Email, order),
            UsersSort::CreatedAt => query.order_by(users::Column::CreatedAt, order),
            UsersSort::UpdatedAt => query.order_by(users::Column::UpdatedAt, order),
        }
    }
}

impl Serialize for UsersSort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            UsersSort::Id => serializer.serialize_str("id"),
            UsersSort::Email => serializer.serialize_str("email"),
            UsersSort::CreatedAt => serializer.serialize_str("created_at"),
            UsersSort::UpdatedAt => serializer.serialize_str("updated_at"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for UsersSort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct UsersSortVisitor;

        impl<'de> serde::de::Visitor<'de> for UsersSortVisitor {
            type Value = UsersSort;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a string representing a UsersSort")
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<UsersSort, E> {
                Ok(match s {
                    "id" => UsersSort::Id,
                    "email" => UsersSort::Email,
                    "created_at" => UsersSort::CreatedAt,
                    "updated_at" => UsersSort::UpdatedAt,
                    _ => return Err(E::invalid_value(serde::de::Unexpected::Str(s), &self)),
                })
            }
        }

        deserializer.deserialize_any(UsersSortVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::UsersSort;
    use serde_json::json;

    #[derive(serde::Deserialize)]
    struct Test {
        _sort: UsersSort,
    }

    #[test]
    fn test_serialize_enum_variants() {
        let variants = vec![
            "id".to_string(),
            "email".to_string(),
            "created_at".to_string(),
            "updated_at".to_string(),
        ];

        for variant in variants {
            let sort = match variant.as_str() {
                "id" => UsersSort::Id,
                "email" => UsersSort::Email,
                "created_at" => UsersSort::CreatedAt,
                "updated_at" => UsersSort::UpdatedAt,
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
            search: None,
            sort: Some(UsersSort::Id),
            order: None,
            limit: None,
            offset: None,
        };

        let serialized = serde_json::to_string(&search).unwrap();

        assert_eq!(
            serialized,
            "{\"search\":null,\"sort\":\"id\",\"order\":null,\"limit\":null,\"offset\":null}"
        );
    }

    #[test]
    fn test_deserialize_search_struct() {
        let search = super::Search {
            search: None,
            sort: Some(UsersSort::Id),
            order: None,
            limit: None,
            offset: None,
        };

        let deserialized = serde_json::from_str::<super::Search>(
            "{\"search\":null,\"sort\":\"id\",\"order\":null,\"limit\":null,\"offset\":null}",
        )
        .unwrap();

        assert_eq!(deserialized, search);
    }
}
