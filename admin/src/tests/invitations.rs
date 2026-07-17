use context::Context;

use crate::data::invitations::{create::Create, search::Search};

#[async_std::test]
async fn test_invite_user() {
    let context: Context = Context::mock_sqlite().await;

    let repository = super::get_repo(&context).await;

    repository
        .invitations()
        .create(Create {
            email: Some("eleven@test.com".to_string()),
            role: None,
            quota: None,
            message: None,
            expires_at: None,
        })
        .await
        .unwrap();

    let paginated = repository
        .invitations()
        .find(Search {
            with_expired: None,
            search: None,
            limit: None,
            offset: None,
            sort: None,
            order: None,
        })
        .await
        .unwrap();
    let invitations = paginated.data;

    assert_eq!(invitations.len(), 1);
    assert_eq!(invitations[0].email, "eleven@test.com");
}

#[async_std::test]
async fn test_expire_invitation() {
    let context: Context = Context::mock_sqlite().await;

    let repository = super::get_repo(&context).await;

    repository
        .invitations()
        .create(Create {
            email: Some("eleven@test.com".to_string()),
            role: None,
            quota: None,
            message: None,
            expires_at: None,
        })
        .await
        .unwrap();

    let paginated = repository
        .invitations()
        .find(Search {
            with_expired: None,
            search: None,
            limit: None,
            offset: None,
            sort: None,
            order: None,
        })
        .await
        .unwrap();
    let invitations = paginated.data;

    assert_eq!(invitations.len(), 1);
    let invitation = invitations.first().unwrap();

    repository
        .invitations()
        .expire(invitation.id)
        .await
        .unwrap();

    let paginated = repository
        .invitations()
        .find(Search {
            with_expired: Some(false),
            search: None,
            limit: None,
            offset: None,
            sort: None,
            order: None,
        })
        .await
        .unwrap();
    let invitations = paginated.data;

    assert_eq!(invitations.len(), 0);
}

#[async_std::test]
async fn test_invitation_rate_limit() {
    let context: Context = Context::mock_sqlite().await;

    let repository = super::get_repo(&context).await;

    repository
        .invitations()
        .create(Create {
            email: Some("first@test.com".to_string()),
            role: None,
            quota: None,
            message: None,
            expires_at: None,
        })
        .await
        .unwrap();

    // A second invitation within the 30s window is refused, even to a different
    // address — the throttle is global anti-spam, not per-recipient.
    let err = repository
        .invitations()
        .create(Create {
            email: Some("second@test.com".to_string()),
            role: None,
            quota: None,
            message: None,
            expires_at: None,
        })
        .await
        .unwrap_err();

    assert!(
        matches!(err, error::Error::TooManyRequests(_)),
        "expected TooManyRequests, got {err:?}"
    );
}
