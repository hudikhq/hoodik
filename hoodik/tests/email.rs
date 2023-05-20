use config::{email::EmailConfig, Config};
use email::{contract::SenderContract, Sender};

#[actix_web::test]
#[ignore = "This test requires a valid smtp server"]
async fn test_send_email_via_smtp() {
    let config = Config::mock();

    let _smtp_credentials = match &config.mailer {
        EmailConfig::Smtp(c) => c,
        _ => panic!("Error loading email config, no smtp setup"),
    };

    let sender = Sender::new(&config).unwrap().unwrap();

    let mut email = sender
        .template(
            "Testing email from ./rust-e2e",
            "If you are seeing this, it works!",
        )
        .unwrap();

    email
        .register_content_template("If you are seeing this, it works!")
        .unwrap();

    sender
        .send(vec![email.to("hello@hudik.eu").unwrap()])
        .await
        .unwrap();
}
