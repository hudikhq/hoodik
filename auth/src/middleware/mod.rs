pub mod load;
pub mod verify;

#[cfg(test)]
mod test {
    use crate::middleware::{load::Load, verify::Verify};
    use actix_web::{web, App};

    #[test]
    fn test_wrap_in_app() {
        let load = Load::new();
        let verify = Verify::new();

        let _app = App::new()
            .wrap(load)
            .wrap(verify)
            .service(web::resource("/").to(|| async { "Hello world!" }));
    }

    #[test]
    fn test_wrap_in_cfg() {
        let load = Load::new();
        let verify = Verify::new();

        let _app = App::new().configure(|cfg| {
            cfg.service(
                web::resource("/")
                    .to(|| async { "Hello world!" })
                    .wrap(load)
                    .wrap(verify),
            );
        });
    }
}
