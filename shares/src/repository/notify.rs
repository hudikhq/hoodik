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

struct Copy {
    subject: &'static str,
    pre_header: &'static str,
    content: &'static str,
}

fn copy(locale: &str) -> Copy {
    match locale {
        "fr" => Copy {
            subject: "Un nouveau fichier a été partagé avec vous sur Hoodik",
            pre_header: "Connectez-vous pour voir ce qui a été partagé avec vous.",
            content: r#"
<h1>Un nouveau fichier a été partagé avec vous sur Hoodik</h1>
<p>
    <strong>{{sender_email}}</strong> a partagé un fichier ou un dossier avec vous.
</p>
<p>
    Le nom du fichier est chiffré de bout en bout — vous seul pouvez le
    déchiffrer après vous être connecté. Vérifiez l'identité de l'expéditeur
    en comparant l'empreinte ci-dessous avec celle qu'il vous a communiquée
    par un autre canal.
</p>
<p>
    Empreinte de l'expéditeur : <code>{{sender_fingerprint}}</code>
</p>
<p>
    <a href="{{link}}" class="btn-primary">Se connecter pour voir</a>
</p>
"#,
        },
        "de" => Copy {
            subject: "Eine neue Datei wurde auf Hoodik mit Ihnen geteilt",
            pre_header: "Melden Sie sich an, um zu sehen, was mit Ihnen geteilt wurde.",
            content: r#"
<h1>Eine neue Datei wurde auf Hoodik mit Ihnen geteilt</h1>
<p>
    <strong>{{sender_email}}</strong> hat eine Datei oder einen Ordner mit Ihnen geteilt.
</p>
<p>
    Der Dateiname ist Ende-zu-Ende-verschlüsselt — nur Sie können ihn nach
    der Anmeldung entschlüsseln. Überprüfen Sie die Identität des Absenders,
    indem Sie den unten stehenden Fingerprint mit dem vergleichen, den er
    Ihnen auf anderem Weg mitgeteilt hat.
</p>
<p>
    Fingerprint des Absenders: <code>{{sender_fingerprint}}</code>
</p>
<p>
    <a href="{{link}}" class="btn-primary">Anmelden und ansehen</a>
</p>
"#,
        },
        "hr" => Copy {
            subject: "Nova datoteka je podijeljena s vama na Hoodiku",
            pre_header: "Prijavite se da vidite što je podijeljeno s vama.",
            content: r#"
<h1>Nova datoteka je podijeljena s vama na Hoodiku</h1>
<p>
    <strong>{{sender_email}}</strong> je podijelio datoteku ili folder s vama.
</p>
<p>
    Naziv datoteke je end-to-end enkriptiran — samo ga vi možete dešifrirati
    nakon prijave. Provjerite identitet pošiljatelja usporedbom fingerprinta
    ispod s onim koji vam je poslao drugim kanalom.
</p>
<p>
    Fingerprint pošiljatelja: <code>{{sender_fingerprint}}</code>
</p>
<p>
    <a href="{{link}}" class="btn-primary">Prijavi se i pogledaj</a>
</p>
"#,
        },
        _ => Copy {
            subject: "You have a new shared file on Hoodik",
            pre_header: "Sign in to view what was shared with you.",
            content: r#"
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
"#,
        },
    }
}

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

    let copy = copy(util::locale::resolve(recipient.locale.as_deref()));

    let mut template = mailer.template(copy.subject, copy.pre_header)?;
    template.add_template_var("sender_email", &sender.email);
    template.add_template_var("sender_fingerprint", &sender.fingerprint);
    template.add_template_var("link", format!("{}/share", ctx.config.get_client_url()));
    template.register_content_template(copy.content)?;
    let template = template.to(&recipient.email)?;

    mailer.send(vec![template]).await?;
    Ok(())
}
