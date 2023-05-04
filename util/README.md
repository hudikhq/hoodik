# Utils

Ideally, we wouldn't have this crate, everything would be in its own crate, but some of these functions are used all around the application
and don't really have a specific place where they should be.

Or in some cases, the crates using them cannot be dependant on each other so we move them here.