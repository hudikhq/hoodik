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
# SMTP_DEFAULT_FROM="Full Name <username@gmail.com>"
```

## Generating application password on Google

Go to this page: https://myaccount.google.com/u/0/apppasswords

Login with your Google account that you will use for sending emails, generate application password 
and put it in your env configuration. Start using the application.

*Gmail SMTP might not work without application password, so give that a try if you encounter any problems*