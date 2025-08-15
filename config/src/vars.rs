use std::str::FromStr;

use clap::{builder::Str, Arg, ArgMatches, Command};
use dotenv::{from_path, vars};

pub(crate) trait GetterType: FromStr + Clone + Send + Sync + 'static {}
impl<T> GetterType for T where T: FromStr + Clone + Send + Sync + 'static {}

pub(crate) trait OptionLike {
    fn is_some(&self) -> bool;
}

pub(crate) struct Var<T: GetterType>(Option<T>);

impl<T> OptionLike for Var<T>
where
    T: GetterType,
{
    fn is_some(&self) -> bool {
        self.0.is_some()
    }
}

pub(crate) struct MaybeVar<T: GetterType>(Option<T>);

impl<T> OptionLike for MaybeVar<T>
where
    T: GetterType,
{
    fn is_some(&self) -> bool {
        self.0.is_some()
    }
}

/// Variable holder
pub(crate) struct Getter<T: OptionLike> {
    inner: T,
}

impl<V> OptionLike for Getter<V>
where
    V: OptionLike,
{
    fn is_some(&self) -> bool {
        self.inner.is_some()
    }
}

impl<V> Getter<V>
where
    V: OptionLike,
{
    pub(crate) fn new(inner: V) -> Self {
        Self { inner }
    }

    /// Create the infallible type that will panic if the value is not set, but we are hoping
    /// to not get to that point
    pub(crate) fn infallible<T: GetterType>(value: Option<T>) -> Getter<Var<T>> {
        Getter::new(Var(value))
    }

    /// Create the fallible type that will not panic if the value is not set
    /// because it will return an option
    pub(crate) fn fallible<T: GetterType>(value: Option<T>) -> Getter<MaybeVar<T>> {
        Getter::new(MaybeVar(value))
    }
}

impl<T> Getter<Var<T>>
where
    T: GetterType,
{
    /// # Panics if the value is missing
    pub(crate) fn get(self) -> T {
        self.inner.0.unwrap()
    }
}

impl<T> Getter<MaybeVar<T>>
where
    T: GetterType,
{
    pub(crate) fn maybe_get(self) -> Option<T> {
        self.inner.0
    }
}

pub(crate) struct Vars {
    name: String,
    version: String,
    about: String,
    matches: Option<ArgMatches>,
    errors: Vec<String>,
}

impl Vars {
    pub(crate) fn new(name: &str, version: &str, about: &str) -> Self {
        let mut vars = Self::create(name, version, about);
        vars.dotenv(std::env::var("ENV_FILE").ok());
        vars.arguments();

        vars
    }

    pub(crate) fn env_only(name: &str, version: &str, about: &str) -> Self {
        let vars = Self::create(name, version, about);
        vars.dotenv(std::env::var("ENV_FILE").ok());

        vars
    }

    pub(crate) fn create(name: &str, version: &str, about: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            about: about.to_string(),
            matches: None,
            errors: Vec::new(),
        }
    }

    /// Prints all the errors and panics if there are any
    pub(crate) fn panic_if_errors(&self, location: &str) {
        if !self.errors.is_empty() {
            for error in self.errors.iter() {
                eprintln!("{error}");
            }

            panic!("Shutting down because of errors in {location}")
        }
    }

    /// Get the inner version from Vars.
    pub(crate) fn get_version(&self) -> String {
        self.version.clone()
    }

    /// Make sure to have the variable and set the default if it is not set
    pub(crate) fn var_default<T: GetterType>(&mut self, name: &str, default: T) -> Getter<Var<T>> {
        let env = self.maybe_env_var(name);
        let arg = self.get_match(name);

        if arg.is_some() {
            return Getter::<Var<T>>::infallible(arg);
        }

        if env.is_some() {
            return Getter::<Var<T>>::infallible(env.maybe_get());
        }

        Getter::<Var<T>>::infallible(Some(default))
    }

    /// Get the variable and fail if it isn't found anywhere
    pub(crate) fn var<T: GetterType>(&mut self, name: &str) -> Getter<Var<T>> {
        let env = self.maybe_env_var(name);
        let arg = self.get_match(name);

        if arg.is_some() {
            return Getter::<Var<T>>::infallible(arg);
        }

        if env.is_some() {
            return Getter::<Var<T>>::infallible(env.maybe_get());
        }

        self.errors.push(format!("{name} is not set"));

        Getter::<Var<T>>::infallible(None)
    }

    /// Get the variable if it is set in the env or in the command line arguments
    pub(crate) fn maybe_var<T: GetterType>(&mut self, name: &str) -> Getter<MaybeVar<T>> {
        let env = self.maybe_env_var(name);
        let arg = self.get_match(name);

        if arg.is_some() {
            return Getter::<MaybeVar<T>>::fallible(arg);
        }

        if env.is_some() {
            return Getter::<MaybeVar<T>>::fallible(env.maybe_get());
        }

        Getter::<MaybeVar<T>>::fallible(None)
    }

    /// Maybe get variable from the env
    fn maybe_env_var<T: GetterType>(&mut self, name: &str) -> Getter<MaybeVar<T>> {
        let value = std::env::var(name);

        if value.is_err() {
            return Getter::<MaybeVar<T>>::fallible(None);
        }

        let value = value.unwrap();

        if value.is_empty() {
            return Getter::<MaybeVar<T>>::fallible(None);
        }

        match value.parse::<T>() {
            Ok(v) => Getter::<MaybeVar<T>>::fallible(Some(v)),
            Err(_e) => {
                self.errors.push(format!(
                    "ENV->{}: Parsing into '{}' failed",
                    name,
                    stringify!(T)
                ));

                Getter::<MaybeVar<T>>::fallible(None)
            }
        }
    }

    /// Get variable from the env
    #[allow(dead_code)]
    fn env_var<T: GetterType>(&mut self, name: &str) -> Getter<Var<T>> {
        let mut value: Option<T> = None;

        if let Ok(v) = std::env::var(name) {
            if v.is_empty() {
                self.errors.push(format!("ENV->{name}: is empty"));
            } else {
                value = match v.parse::<T>() {
                    Ok(v) => Some(v),
                    Err(_e) => {
                        self.errors
                            .push(format!("ENV->{name}: Parsing into type failed"));

                        None
                    }
                };
            }
        } else {
            self.errors.push(format!("{name} env is not set"));
        }

        Getter::<Var<T>>::infallible(value)
    }

    /// Attempt to get the variable from the command line arguments
    fn get_match<T: GetterType>(&mut self, name: &str) -> Option<T> {
        let arg = self
            .matches
            .as_ref()
            .and_then(|m| m.try_get_one::<String>(name).ok())
            .unwrap_or(None)
            .cloned();

        let mut value: Option<T> = None;

        if let Some(v) = arg {
            if !v.is_empty() {
                value = match v.parse::<T>() {
                    Ok(v) => Some(v),
                    Err(_e) => {
                        self.errors.push(format!(
                            "ARG->{}: Parsing into '{}' failed",
                            name,
                            stringify!(T)
                        ));

                        None
                    }
                };
            }
        }

        value
    }

    /// Loads the env variables from the provided path
    pub(crate) fn dotenv(&self, path: Option<String>) {
        let vars: Vec<(String, String)> = match path {
            Some(p) => {
                match from_path(&p) {
                    Ok(_) => (),
                    Err(e) => panic!("Couldn't load the dotenv config at '{p}', error: {e}"),
                }

                vars().collect()
            }
            None => vars().collect(),
        };

        for (key, value) in vars.iter() {
            std::env::set_var(key, value);
        }
    }

    /// Define matches from the command line arguments
    pub(crate) fn arguments(&mut self) {
        let command = Command::new(self.name.clone())
        .version(Str::from(self.version.clone()))
        .about(self.about.clone())
        .arg(
            Arg::new("port")
                .id("HTTP_PORT")
                .short('p')
                .long("port")
                .help("HTTP port where this application will listen")
                .required(false),
        )
        .arg(
            Arg::new("address")
                .id("HTTP_ADDRESS")
                .short('a')
                .long("address")
                .help("HTTP address where the application will attach itself")
                .required(false),
        )
        .arg(
            Arg::new("data_dir")
                .id("DATA_DIR")
                .short('d')
                .long("data-dir")
                .help("Location where the application will store the data")
                .required(false),
        )
        .arg(
            Arg::new("database_url")
                .id("DATABASE_URL")
                .long("pg-url")
                .help("Connection string for the postgres database, by default we will fallback to sqlite database stored in your data-dir")
                .required(false),
        )
        .arg(
            Arg::new("log")
                .id("RUST_LOG")
                .short('l')
                .long("log")
                .help("Set the RUST_LOG variable")
                .required(false),
        );

        self.matches = Some(command.get_matches());
    }

    #[cfg(test)]
    fn clone_errors(&mut self) -> Vec<String> {
        self.errors.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_getter() {
        let getter = Getter::<Var<u16>>::infallible(Some(8080));
        assert_eq!(getter.get(), 8080);

        let getter = Getter::<MaybeVar<u16>>::fallible(Some(8080));
        assert_eq!(getter.maybe_get(), Some(8080));

        let getter = Getter::<MaybeVar<u16>>::fallible(None::<u16>);
        assert_eq!(getter.maybe_get(), None);
    }

    #[test]
    #[should_panic]
    fn test_getter_fails() {
        let getter = Getter::<Var<u16>>::infallible(None::<u16>);
        getter.get(); // Panics
    }

    #[test]
    fn test_vars_env() {
        let mut vars = Vars::create("test", "0.1.0", "test");

        std::env::set_var("HTTP_PORT__TEST_1", "8080");
        let getter = vars.env_var::<u16>("HTTP_PORT__TEST_1");
        assert_eq!(getter.get(), 8080);

        std::env::set_var("HTTP_PORT__TEST_2", "8080");
        let getter = vars.maybe_env_var::<u16>("HTTP_PORT__TEST_2");
        assert_eq!(getter.maybe_get(), Some(8080));

        let getter = vars.maybe_env_var::<u16>("HTTP_PORT_2");
        assert_eq!(getter.maybe_get(), None);
    }

    #[test]
    #[should_panic]
    fn test_vars_fails_invalid_env_type() {
        let mut vars = Vars::create("test", "0.1.0", "test");

        std::env::set_var("HTTP_PORT__TEST_3", "abc");
        let _getter = vars.env_var::<u16>("HTTP_PORT__TEST_3");
        let errors = vars.clone_errors();
        assert_eq!(errors.len(), 1);
        vars.panic_if_errors("test");
    }

    #[test]
    #[should_panic]
    fn test_vars_fails_on_empty_env() {
        let mut vars = Vars::create("test", "0.1.0", "test");

        std::env::set_var("HTTP_PORT__TEST_5", "");
        let _getter = vars.env_var::<u16>("HTTP_PORT__TEST_5");
        let errors = vars.clone_errors();
        assert_eq!(errors.len(), 1);
        vars.panic_if_errors("test");
    }
}
