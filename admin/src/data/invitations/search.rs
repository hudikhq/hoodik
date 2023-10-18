use entity::{invitations, sort::Sortable, Order, QueryOrder, Select};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Search {
    pub with_expired: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub sort: Option<InvitationsSort>,
    pub order: Option<String>,
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
pub enum InvitationsSort {
    Id,
    Email,
    #[default]
    CreatedAt,
    ExpiresAt,
}

impl Sortable for InvitationsSort {
    type Entity = invitations::Entity;

    fn sort(&self, query: Select<Self::Entity>, order: Order) -> Select<Self::Entity> {
        match self {
            InvitationsSort::Id => query.order_by(invitations::Column::Id, order),
            InvitationsSort::Email => query.order_by(invitations::Column::Email, order),
            InvitationsSort::CreatedAt => query.order_by(invitations::Column::CreatedAt, order),
            InvitationsSort::ExpiresAt => query.order_by(invitations::Column::ExpiresAt, order),
        }
    }
}

impl Serialize for InvitationsSort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            InvitationsSort::Id => serializer.serialize_str("id"),
            InvitationsSort::Email => serializer.serialize_str("email"),
            InvitationsSort::CreatedAt => serializer.serialize_str("created_at"),
            InvitationsSort::ExpiresAt => serializer.serialize_str("expires_at"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for InvitationsSort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct InvitationsSortVisitor;

        impl<'de> serde::de::Visitor<'de> for InvitationsSortVisitor {
            type Value = InvitationsSort;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a string representing a InvitationsSort")
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<InvitationsSort, E> {
                Ok(match s {
                    "id" => InvitationsSort::Id,
                    "email" => InvitationsSort::Email,
                    "created_at" => InvitationsSort::CreatedAt,
                    "expires_at" => InvitationsSort::ExpiresAt,
                    _ => return Err(E::invalid_value(serde::de::Unexpected::Str(s), &self)),
                })
            }
        }

        deserializer.deserialize_any(InvitationsSortVisitor)
    }
}

#[cfg(test)]
mod test {
    use super::InvitationsSort;
    use serde_json::json;

    #[derive(serde::Deserialize)]
    struct Test {
        _sort: InvitationsSort,
    }

    #[test]
    fn test_serialize_enum_variants() {
        let variants = vec![
            "id".to_string(),
            "email".to_string(),
            "created_at".to_string(),
            "expires_at".to_string(),
        ];

        for variant in variants {
            let sort = match variant.as_str() {
                "id" => InvitationsSort::Id,
                "email" => InvitationsSort::Email,
                "created_at" => InvitationsSort::CreatedAt,
                "expires_at" => InvitationsSort::ExpiresAt,
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
            search: None,
            sort: Some(InvitationsSort::Id),
            order: None,
            limit: None,
            offset: None,
        };

        let serialized = serde_json::to_string(&search).unwrap();

        assert_eq!(
            serialized,
            "{\"with_expired\":null,\"search\":null,\"limit\":null,\"offset\":null,\"sort\":\"id\",\"order\":null}"
        );
    }

    #[test]
    fn test_deserialize_search_struct() {
        let search = super::Search {
            with_expired: None,
            search: None,
            sort: Some(InvitationsSort::Id),
            order: None,
            limit: None,
            offset: None,
        };

        let deserialized = serde_json::from_str::<super::Search>(
            "{\"with_expired\":null,\"search\":null,\"sort\":\"id\",\"order\":null,\"limit\":null,\"offset\":null}",
        )
        .unwrap();

        assert_eq!(deserialized, search);
    }
}
