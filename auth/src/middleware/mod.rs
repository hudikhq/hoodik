pub mod verify;

#[cfg(test)]
mod test {
    use super::verify::Verify;
    use actix_web::{web, App};

    #[test]
    fn test_wrap_in_app() {
        let verify = Verify::default();
        let refresh = Verify::new_refresh();

        let _app = App::new()
            .wrap(verify)
            .wrap(refresh)
            .service(web::resource("/").to(|| async { "Hello world!" }));
    }

    #[test]
    fn test_wrap_in_cfg() {
        let verify = Verify::new_refresh();
        let refresh = Verify::new_refresh();

        let _app = App::new().configure(|cfg| {
            cfg.service(
                web::resource("/")
                    .to(|| async { "Hello world!" })
                    .wrap(verify)
                    .wrap(refresh),
            );
        });
    }
}
