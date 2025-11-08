# Email

This crate provides email sending capabilities to the application.

Currently, only SMTP is supported.

## SMTP Configuration

In order to setup SMTP, its best to use some reputable service to send emails (Gmail for example).

You will need to provide SMTP credentials through the `.env` file, or the ENV given to docker.

```env
# MAILER_TYPE=smtp
# SMTP_ADDRESS=smtp.gmail.com
# SMTP_USERNAME="username@gmail.com"
# SMTP_PASSWORD="generated-app-password"
# SMTP_PORT=465 # Optional, default: 465
# SMTP_TLS_MODE=implicit # Optional, values: starttls, implicit, none - auto-detected from port if not set
# SMTP_DEFAULT_FROM_EMAIL="username@gmail.com"
# SMTP_DEFAULT_FROM_NAME="Full Name" # Optional
# SMTP_DEFAULT_FROM="Full Name <username@gmail.com>" # DEPRECATED: Use SMTP_DEFAULT_FROM_EMAIL and SMTP_DEFAULT_FROM_NAME instead
```

## TLS Modes

The `SMTP_TLS_MODE` setting determines how TLS encryption is used for the SMTP connection:

- **`starttls`** - STARTTLS encryption (typically used with port 587)
- **`implicit`** - Implicit TLS/SSL encryption (typically used with port 465) - Default for Gmail
- **`none`** - No encryption (typically port 25, development only - not recommended for production)

If `SMTP_TLS_MODE` is not specified, it will be auto-detected based on the port:
- Port 587 → STARTTLS
- Port 465 → Implicit TLS
- Port 25 → None
- Other ports → Implicit TLS (default)

## Generating application password on Google

Go to this page: https://myaccount.google.com/u/0/apppasswords

Login with your Google account that you will use for sending emails, generate application password 
and put it in your env configuration. Start using the application.

*Gmail SMTP might not work without application password, so give that a try if you encounter any problems*