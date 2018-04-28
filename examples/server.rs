extern crate gotham;
extern crate handlebars_gotham as hbs;
extern crate hyper;
#[macro_use]
extern crate maplit;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use gotham::state::State;
use gotham::http::response::create_response;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;

use hbs::{DirectorySource, HandlebarsEngine, MemorySource, Template};
use hbs::handlebars::{to_json, Handlebars, Helper, RenderContext, RenderError};
use hyper::{Response, StatusCode};
use serde_json::value::{Map, Value};

#[derive(Serialize, Debug)]
pub struct Team {
    name: String,
    pts: u16,
}

pub fn make_data() -> Map<String, Value> {
    let mut data = Map::new();

    data.insert("year".to_string(), to_json(&"2017".to_owned()));

    let teams = vec![
        Team {
            name: "Jiangsu Sainty".to_string(),
            pts: 43u16,
        },
        Team {
            name: "Beijing Guoan".to_string(),
            pts: 27u16,
        },
        Team {
            name: "Guangzhou Evergrand".to_string(),
            pts: 22u16,
        },
        Team {
            name: "Shandong Luneng".to_string(),
            pts: 12u16,
        },
    ];

    data.insert("teams".to_string(), to_json(&teams));
    data.insert("engine".to_string(), to_json(&"serde_json".to_owned()));
    data
}

/// the handlers
fn index(mut state: State) -> (State, Response) {
    state.put(Template::new("some/path/hello", make_data()));

    let res = create_response(&state, StatusCode::Ok, None);

    (state, res)
}

fn memory(mut state: State) -> (State, Response) {
    state.put(Template::new("memory", make_data()));

    let res = create_response(&state, StatusCode::Ok, None);

    (state, res)
}

fn temp(mut state: State) -> (State, Response) {
    state.put(Template::with(
        include_str!("templates/some/path/hello.hbs"),
        make_data(),
    ));

    let res = create_response(&state, StatusCode::Ok, None);

    (state, res)
}

fn plain(state: State) -> (State, Response) {
    let res = create_response(
        &state,
        StatusCode::Ok,
        Some(("It works".as_bytes().to_owned(), mime::TEXT_PLAIN)),
    );

    (state, res)
}

fn main() {
    let mem_templates = btreemap! {
        "memory".to_owned() => include_str!("templates/some/path/hello.hbs").to_owned()
    };

    let hbse = HandlebarsEngine::new(vec![
        Box::new(DirectorySource::new("./examples/templates/", ".hbs")),
        Box::new(MemorySource(mem_templates)),
    ]);

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    hbse.handlebars_mut().register_helper(
        "some_helper",
        Box::new(
            |_: &Helper, _: &Handlebars, _: &mut RenderContext| -> Result<(), RenderError> {
                Ok(())
            },
        ),
    );

    let (chain, pipelines) = single_pipeline(new_pipeline().add(hbse).build());

    let router = build_router(chain, pipelines, |route| {
        route.get("/").to(index);
        route.get("/memory").to(memory);
        route.get("/temp").to(temp);
        route.get("/plain").to(plain);
    });

    let addr = "127.0.0.1:7878";

    println!("Listening on http://{}", addr);
    gotham::start(addr, router);
}
