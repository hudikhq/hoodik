use error::{AppResult, Error};
use handlebars::Handlebars;
use lettre::{
    message::{header::ContentType, Mailbox, MessageBuilder},
    Message,
};
use std::collections::BTreeMap;

pub struct Template {
    pub base: Handlebars<'static>,
    pub data: BTreeMap<String, String>,
    has_to: bool,
    has_from: bool,
    base_content: bool,
    has_reply_to: bool,
    base_extra_head: bool,
    skip_send: bool,
    builder: MessageBuilder,
}

impl Template {
    /// Create base template with inserted base HTML for the email
    /// Provide it with subject (for the title) and pre_header (for the text that will display as preview)
    pub fn new(subject: &str, pre_header: &str) -> AppResult<Self> {
        let mut base = Handlebars::new();
        base.register_template_string("__base_template", include_str!("../assets/base.hbs"))?;

        let mut data = BTreeMap::new();
        data.insert("base_subject".to_string(), subject.to_string());
        data.insert("base_pre_header".to_string(), pre_header.to_string());
        data.insert("base_app_name".to_string(), "Hoodik".to_string());
        data.insert("base_app_version".to_string(), "unknown".to_string());

        let builder = Message::builder().subject(subject);

        Ok(Self {
            base,
            data,
            has_to: false,
            has_from: false,
            base_content: false,
            has_reply_to: false,
            base_extra_head: false,
            skip_send: false,
            builder,
        })
    }

    /// Add any kind of variable to the template
    pub fn add_template_var<T: ToString>(&mut self, key: &str, value: T) {
        self.data.insert(key.to_string(), value.to_string());
    }

    /// Add additional `<head>` data into the base email template html
    pub fn register_extra_head_template(&mut self, content: &str) -> AppResult<()> {
        self.base
            .register_template_string("__base_extra_head", content)?;

        self.base_extra_head = true;

        Ok(())
    }

    /// Add handlebars template to the base template that will serve as
    /// the content of the email.
    /// This can be a simple string or a string with handlebars syntax
    /// that has access to all the data provided through the add_template_var method
    pub fn register_content_template(&mut self, content: &str) -> AppResult<&mut Self> {
        self.base
            .register_template_string("__base_content", content)?;

        self.base_content = true;

        Ok(self)
    }

    /// Add a sender to the email
    pub fn from(mut self, from: &str) -> AppResult<Self> {
        let mailbox = from
            .parse()
            .map_err(|_| Error::BadRequest(format!("invalid_from_address_provided:{from}")))?;

        self.builder = self.builder.from(mailbox);

        self.has_from = true;

        Ok(self)
    }

    /// Add a sender to the email
    pub fn from_mailbox(mut self, from: &Mailbox) -> Self {
        self.builder = self.builder.from(from.clone());

        self.has_from = true;

        self
    }

    /// Add a reply_to field on the email
    pub fn reply_to(mut self, reply_to: &str) -> AppResult<Self> {
        let mailbox = reply_to.parse().map_err(|_| {
            Error::BadRequest(format!("invalid_reply_to_address_provided:{reply_to}"))
        })?;

        self.builder = self.builder.reply_to(mailbox);

        self.has_reply_to = true;

        Ok(self)
    }

    /// Add a reply to field on the email
    pub fn reply_to_mailbox(mut self, reply_to: &Mailbox) -> Self {
        self.builder = self.builder.reply_to(reply_to.clone());

        self.has_reply_to = true;

        self
    }

    /// Add a recipient of the email
    pub fn to(self, to: &str) -> AppResult<Self> {
        let mailbox = to
            .parse()
            .map_err(|_| Error::BadRequest(format!("invalid_to_address_provided:{to}")))?;

        Ok(self.to_mailbox(&mailbox))
    }

    /// Add a recipient to the email
    pub fn to_mailbox(mut self, to: &Mailbox) -> Self {
        self.builder = self.builder.to(to.clone());

        if to.email.domain() == "test.com" {
            self.skip_send = true;
        }

        self.has_to = true;

        self
    }

    /// Generate the final HTML of the email
    pub fn render(&self) -> AppResult<String> {
        self.base
            .render("__base_template", &self.data)
            .map_err(Error::from)
    }

    /// Does the current template have a from field defined
    pub fn has_from(&self) -> bool {
        self.has_from
    }

    /// Is this email marked to not be sent
    pub fn skip_send(&self) -> bool {
        self.skip_send
    }

    /// Does the current template have a to field defined
    pub fn has_to(&self) -> bool {
        self.has_to
    }

    /// Does the current template have a reply_to field defined
    pub fn has_reply_to(&self) -> bool {
        self.has_reply_to
    }

    /// Generate the final email message
    pub fn message(&self) -> AppResult<Message> {
        let html = self.render()?;

        self.builder
            .clone()
            .header(ContentType::TEXT_HTML)
            .body(html)
            .map_err(Error::from)
    }
}

#[cfg(test)]
mod test {
    use super::Template;

    #[test]
    fn template_can_be_created() {
        let template = Template::new("subject", "pre_header").unwrap();

        assert!(!template.base_content);
    }

    #[test]
    fn template_can_be_given_base_content() {
        let mut template = Template::new("subject", "pre_header").unwrap();

        template
            .register_content_template("Some Extra Content Template {{ arbitrary_var }}")
            .unwrap();

        assert!(template.base_content);
    }

    #[test]
    fn template_will_render_and_have_the_extra_content_var() {
        let mut template = Template::new("subject", "pre_header").unwrap();

        template
            .register_content_template("Some Extra Content Template {{ arbitrary_var }}")
            .unwrap();

        template.add_template_var("arbitrary_var", "---this is the extra content---");

        let html = template.render().unwrap();

        assert!(html.contains("---this is the extra content---"));
    }

    #[test]
    fn template_fails_to_build_message_without_from() {
        let template = Template::new("subject", "pre_header").unwrap();

        let message = template.message();

        assert_eq!(format!("{:?}", message), "Err(LettreError(MissingFrom))");
    }

    #[test]
    fn template_fails_to_build_message_without_to() {
        let mut template = Template::new("subject", "pre_header").unwrap();

        template = template.from("test@email.com").unwrap();

        let message = template.message();

        assert_eq!(format!("{:?}", message), "Err(LettreError(MissingTo))");
    }

    #[test]
    fn template_can_have_multiple_recipients() {
        let mut template = Template::new("subject", "pre_header").unwrap();

        template = template.from("from@email.com").unwrap();
        template = template.to("to1@email.com").unwrap();
        template = template.to("to2@email.com").unwrap();

        let message = template.message();

        assert!(message.is_ok());
    }
}
