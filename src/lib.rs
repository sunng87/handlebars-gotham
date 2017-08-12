pub extern crate handlebars;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate serde;
extern crate serde_json;
extern crate hyper;
extern crate futures;
extern crate mime;
extern crate walkdir;
#[macro_use]
extern crate log;

pub use self::middleware::Template;
pub use self::middleware::HandlebarsEngine;
pub use self::sources::{Source, SourceError};
pub use self::sources::directory::DirectorySource;
pub use self::sources::memory::MemorySource;

mod middleware;
mod sources;
