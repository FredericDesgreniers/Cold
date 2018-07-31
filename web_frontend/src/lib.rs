extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate rand;

pub mod ws_update;

use actix::Addr;
use actix::Arbiter;
use actix_web::fs;
use actix_web::ws;
use actix_web::HttpResponse;
use actix_web::{server, App, HttpRequest};
use ws_update::UpdateServer;

/// Update websocket root for the front-end
pub fn update_route(
    req: &HttpRequest<ws_update::WsUpdateSessionState>,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(req, ws_update::WsUpdateSession::default())
}

/// Start the front-end server
pub fn start_server() -> Addr<UpdateServer> {
    println!("Starting frontend...");

    let _ = env_logger::init();

    let update_server = Arbiter::start(|_ctx| UpdateServer::default());

    {
        let update_server = update_server.clone();
        server::new(move || {
            let state = ws_update::WsUpdateSessionState::new(update_server.clone());
            vec![
                App::with_state(state)
                    .prefix("/ws")
                    .resource("/update/", |r| r.route().f(update_route))
                    .boxed(),
                App::new()
                    .handler(
                        "/",
                        fs::StaticFiles::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static/"))
                            .unwrap()
                            .index_file("index.html"),
                    )
                    .boxed(),
            ]
        }).bind("127.0.0.1:80")
            .unwrap()
            .start();
    }
    update_server
}
