use std::sync::{RwLock, RwLockWriteGuard, Arc};
use std::io;
use std::panic::RefUnwindSafe;

use handlebars::{Handlebars, TemplateRenderError, to_json};

use gotham::handler::HandlerFuture;
use gotham::state::State;
use gotham::middleware::{Middleware, NewMiddleware};
use gotham::http::response;

use mime;
use hyper::StatusCode;
use futures::{future, Future};

use serde::ser::Serialize as ToJson;
use serde_json::value::Value as Json;

use sources::{Source, SourceError};

#[derive(StateData)]
pub struct Template {
    name: Option<String>,
    content: Option<String>,
    value: Json,
}

impl Template {
    /// render some template from pre-registered templates
    pub fn new<T: ToJson>(name: &str, value: T) -> Template {
        Template {
            name: Some(name.to_string()),
            value: to_json(&value),
            content: None,
        }
    }

    /// render some template with temporary template string
    pub fn with<T: ToJson>(content: &str, value: T) -> Template {
        Template {
            name: None,
            value: to_json(&value),
            content: Some(content.to_string()),
        }
    }
}

/// The handlebars template engine
#[derive(Clone)]
pub struct HandlebarsEngine {
    pub sources: Arc<Vec<Box<Source + Send + Sync + RefUnwindSafe>>>,
    pub registry: Arc<RwLock<Box<Handlebars>>>,
}

impl HandlebarsEngine {
    /// create a handlebars template engine
    pub fn new(sources: Vec<Box<Source + Send + Sync + RefUnwindSafe>>) -> HandlebarsEngine {
        HandlebarsEngine {
            sources: Arc::new(sources),
            registry: Arc::new(RwLock::new(Box::new(Handlebars::new()))),
        }
    }

    /// create a handlebars template engine from existed handlebars registry
    pub fn from(
        reg: Handlebars,
        sources: Vec<Box<Source + Send + Sync + RefUnwindSafe>>,
    ) -> HandlebarsEngine {
        HandlebarsEngine {
            sources: Arc::new(sources),
            registry: Arc::new(RwLock::new(Box::new(reg))),
        }
    }

    /// load template from registered sources
    pub fn reload(&self) -> Result<(), SourceError> {
        let mut hbs = self.handlebars_mut();
        hbs.clear_templates();
        for s in self.sources.iter() {
            try!(s.load(&mut hbs))
        }
        Ok(())
    }

    /// access internal handlebars registry, useful to register custom helpers
    pub fn handlebars_mut(&self) -> RwLockWriteGuard<Box<Handlebars>> {
        self.registry.write().unwrap()
    }
}

impl NewMiddleware for HandlebarsEngine {
    type Instance = HandlebarsEngine;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl Middleware for HandlebarsEngine {
    fn call<Chain>(self, state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        let f = chain(state)
            .and_then(move |(state, mut response)| {
                if let Some(h) = state.try_borrow::<Template>() {
                    let hbs = self.registry.read().unwrap();
                    let page_wrapper = if let Some(ref name) = h.name {
                        Some(hbs.render(name, &h.value).map_err(
                            TemplateRenderError::from,
                        ))
                    } else if let Some(ref content) = h.content {
                        Some(hbs.render_template(content, &h.value))
                    } else {
                        None
                    };

                    if let Some(page_result) = page_wrapper {
                        match page_result {
                            Ok(page) => {
                                response::extend_response(
                                    &state,
                                    &mut response,
                                    StatusCode::Ok,
                                    Some((page.into_bytes(), mime::TEXT_HTML)),
                                );
                            }
                            Err(_) => {
                                //TODO
                            }
                        }
                    }
                }
                future::ok((state, response))
            });
        Box::new(f)
    }
}
