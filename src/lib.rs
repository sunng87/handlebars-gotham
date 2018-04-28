extern crate futures;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
pub extern crate handlebars;
extern crate hyper;
#[macro_use]
extern crate log;
extern crate mime;
extern crate serde;
extern crate serde_json;
extern crate walkdir;

pub use self::middleware::Template;
pub use self::middleware::HandlebarsEngine;
pub use self::sources::{Source, SourceError};
pub use self::sources::directory::DirectorySource;
pub use self::sources::memory::MemorySource;

mod middleware;
mod sources;
