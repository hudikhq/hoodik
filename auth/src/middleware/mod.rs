pub(crate) mod refresh;

#[cfg(test)]
mod test {
    use super::refresh::Refresh;
    use actix_web::{web, App};

    #[test]
    fn test_wrap_in_app() {
        let refresh = Refresh::default();

        let _app = App::new()
            .wrap(refresh)
            .service(web::resource("/").to(|| async { "Hello world!" }));
    }

    #[test]
    fn test_wrap_in_cfg() {
        let refresh = Refresh::default();

        let _app = App::new().configure(|cfg| {
            cfg.service(
                web::resource("/")
                    .to(|| async { "Hello world!" })
                    .wrap(refresh),
            );
        });
    }
}
