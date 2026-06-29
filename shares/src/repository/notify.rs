//! Best-effort share-notification email dispatch.
//! Honours the recipient's `share_notifications_enabled` toggle and the
//! deployment's mailer config — when neither is set up, the call is a
//! silent no-op so a missing SMTP relay never blocks a share that the
//! server otherwise accepted.
//!
//! The body deliberately omits the file name. Filenames are end-to-end
//! encrypted; the server only ever sees ciphertext, so leaking the
//! recipient's plaintext name through an outbound email would defeat
//! the guarantee. The link points at the share hub — the recipient
//! signs in and decrypts the name with their private key.

use context::{Context, SenderContract};
use entity::users;
use error::AppResult;

const SUBJECT: &str = "You have a new shared file on Hoodik";
const PRE_HEADER: &str = "Sign in to view what was shared with you.";
const TEMPLATE: &str = r#"
<h1>You have a new shared file on Hoodik</h1>
<p>
    <strong>{{sender_email}}</strong> has shared a file or folder with you.
</p>
<p>
    The file name is end-to-end encrypted — only you can decrypt it after
    you sign in. Verify the sender's identity by checking that the fingerprint
    below matches the one they shared with you out of band.
</p>
<p>
    Sender fingerprint: <code>{{sender_fingerprint}}</code>
</p>
<p>
    <a href="{{link}}" class="btn-primary">Sign in to view</a>
</p>
"#;

/// Send one notification email if both the deployment mailer and the
/// recipient's preference allow it. Failures are logged and swallowed —
/// the share row is already committed and a flaky SMTP relay must not
/// roll it back.
pub(crate) async fn share_created(
    ctx: &Context,
    sender: &users::Model,
    recipient: &users::Model,
) {
    if !recipient.share_notifications_enabled {
        return;
    }
    if let Err(e) = dispatch(ctx, sender, recipient).await {
        log::warn!(
            "share notification to {} failed: {}",
            recipient.email,
            e
        );
    }
}

async fn dispatch(
    ctx: &Context,
    sender: &users::Model,
    recipient: &users::Model,
) -> AppResult<()> {
    let mailer = match &ctx.sender {
        Some(s) => s,
        None => return Ok(()),
    };

    let mut template = mailer.template(SUBJECT, PRE_HEADER)?;
    template.add_template_var("sender_email", &sender.email);
    template.add_template_var("sender_fingerprint", &sender.fingerprint);
    template.add_template_var("link", format!("{}/share", ctx.config.get_client_url()));
    template.register_content_template(TEMPLATE)?;
    let template = template.to(&recipient.email)?;

    mailer.send(vec![template]).await?;
    Ok(())
}
