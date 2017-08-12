extern crate gotham;
extern crate handlebars_gotham as hbs;
extern crate hyper;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate maplit;
extern crate mime;

use gotham::state::State;
use gotham::http::response::create_response;
use gotham::handler::{NewHandlerService, NewHandler};
use gotham::middleware::pipeline::new_pipeline;
use gotham::router::Router;
use gotham::router::route::{Extractors, Route, RouteImpl, Delegation};
use gotham::router::route::dispatch::{new_pipeline_set, finalize_pipeline_set, PipelineSet,
                                      PipelineHandleChain, DispatcherImpl};
use gotham::router::route::matcher::MethodOnlyRouteMatcher;
use gotham::router::request::path::NoopPathExtractor;
use gotham::router::request::query_string::NoopQueryStringExtractor;
use gotham::router::response::finalizer::ResponseFinalizerBuilder;
use gotham::router::tree::TreeBuilder;
use gotham::router::tree::node::{NodeBuilder, SegmentType};

use hbs::{Template, HandlebarsEngine, DirectorySource, MemorySource};
use hbs::handlebars::{Handlebars, RenderContext, RenderError, Helper, to_json};
use hyper::server::Http;
use hyper::{Method, Request, Response, StatusCode};
use serde_json::value::{Value, Map};

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
fn index(mut state: State, _req: Request) -> (State, Response) {
    state.put(Template::new("some/path/hello", make_data()));

    let res = create_response(&state, StatusCode::Ok, None);

    (state, res)
}

fn memory(mut state: State, _req: Request) -> (State, Response) {
    state.put(Template::new("memory", make_data()));

    let res = create_response(&state, StatusCode::Ok, None);

    (state, res)
}

fn temp(mut state: State, _req: Request) -> (State, Response) {
    state.put(Template::with(
        include_str!("templates/some/path/hello.hbs"),
        make_data(),
    ));

    let res = create_response(&state, StatusCode::Ok, None);

    (state, res)
}

fn plain(state: State, _req: Request) -> (State, Response) {
    let res = create_response(
        &state,
        StatusCode::Ok,
        Some(("It works".as_bytes().to_owned(), mime::TEXT_PLAIN)),
    );

    (state, res)
}



fn main() {
    let mem_templates =
        btreemap! {
            "memory".to_owned() => include_str!("templates/some/path/hello.hbs").to_owned()
        };

    let hbse = HandlebarsEngine::new(vec![
        Box::new(
            DirectorySource::new("./examples/templates/", ".hbs")
        ),
        Box::new(MemorySource(mem_templates)),
    ]);

    // load templates from all registered sources
    if let Err(r) = hbse.reload() {
        panic!("{}", r);
    }

    hbse.handlebars_mut().register_helper(
        "some_helper",
        Box::new(|_: &Helper,
         _: &Handlebars,
         _: &mut RenderContext|
         -> Result<(), RenderError> {
            Ok(())
        }),
    );

    // -________________________- start
    let mut tree_builder = TreeBuilder::new();
    let ps_builder = new_pipeline_set();
    let (ps_builder, global) = ps_builder.add(new_pipeline().add(hbse).build());
    let ps = finalize_pipeline_set(ps_builder);

    tree_builder.add_route(static_route(
        vec![Method::Get],
        || Ok(index),
        (global, ()),
        ps.clone(),
    ));

    let mut memory_handler = NodeBuilder::new("memory", SegmentType::Static);
    memory_handler.add_route(static_route(
        vec![Method::Get],
        || Ok(memory),
        (global, ()),
        ps.clone(),
    ));
    tree_builder.add_child(memory_handler);

    let mut temp_handler = NodeBuilder::new("temp", SegmentType::Static);
    temp_handler.add_route(static_route(
        vec![Method::Get],
        || Ok(temp),
        (global, ()),
        ps.clone(),
    ));
    tree_builder.add_child(temp_handler);

    let mut plain_handler = NodeBuilder::new("plain", SegmentType::Static);
    plain_handler.add_route(static_route(
        vec![Method::Get],
        || Ok(plain),
        (global, ()),
        ps.clone(),
    ));
    tree_builder.add_child(plain_handler);
    let tree = tree_builder.finalize();

    let response_finalizer_builder = ResponseFinalizerBuilder::new();
    let response_finalizer = response_finalizer_builder.finalize();
    let router = Router::new(tree, response_finalizer);
    // -____________________________- end

    let addr = "127.0.0.1:7878".parse().unwrap();

    let server = Http::new()
        .bind(&addr, NewHandlerService::new(router))
        .unwrap();

    println!("Listening on http://{}", server.local_addr().unwrap());
    server.run().unwrap();
}

// router copied from gotham example
fn static_route<NH, P, C>(
    methods: Vec<Method>,
    new_handler: NH,
    active_pipelines: C,
    ps: PipelineSet<P>,
) -> Box<Route + Send + Sync>
where
    NH: NewHandler + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    // Requests must have used the specified method(s) in order for this Route to match.
    //
    // You could define your on RouteMatcher of course.. perhaps you'd like to only match on
    // requests that are made using the GET method and send a User-Agent header for a particular
    // version of browser you'd like to make fun of....
    let matcher = MethodOnlyRouteMatcher::new(methods);

    // For Requests that match this Route we'll dispatch them to new_handler via the pipelines
    // defined in active_pipelines.
    //
    // n.b. We also specify the set of all known pipelines in the application so the dispatcher can
    // resolve the pipeline references provided in active_pipelines. For this application that is
    // only the global pipeline.
    let dispatcher = DispatcherImpl::new(new_handler, active_pipelines, ps);
    let extractors: Extractors<NoopPathExtractor, NoopQueryStringExtractor> = Extractors::new();
    let route = RouteImpl::new(
        matcher,
        Box::new(dispatcher),
        extractors,
        Delegation::Internal,
    );
    Box::new(route)
}
//
