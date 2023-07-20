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
    let invitation = invitations.get(0).unwrap();

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
