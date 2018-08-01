extern crate actix;
extern crate actix_web;
extern crate env_logger;
extern crate rand;
extern crate cold_data;
extern crate futures;

pub mod ws_update;

use futures::Future;
use actix::Addr;
use actix::Arbiter;
use actix_web::fs;
use actix_web::ws;
use actix_web::HttpResponse;
use actix_web::{server, App, HttpRequest};
use ws_update::UpdateServer;
use cold_data::{DbConnectionPool, models::ListCommands};
use actix_web::{Responder, AsyncResponder};
use actix_web::http;

/// Update websocket root for the front-end
pub fn update_route(
    req: &HttpRequest<ws_update::WsUpdateSessionState>,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(req, ws_update::WsUpdateSession::default())
}

fn commands_route(req: &HttpRequest<ApiState>) -> impl Responder {
    let db: &Addr<DbConnectionPool> = &req.state().db;

    db.send(ListCommands{})
        .from_err()
        .and_then(|result| {
            match result {
                Ok(result) => {
                    Ok(HttpResponse::Ok().json(result))
                },
                Err(err) => {
                    Err(err)
                }
            }
        })
        .responder()

}

struct ApiState {
    db: Addr<DbConnectionPool>
}

/// Start the front-end server
pub fn start_server(db: Addr<DbConnectionPool>) -> Addr<UpdateServer> {
    println!("Starting frontend...");

    let _ = env_logger::init();

    let update_server = Arbiter::start(|_ctx| UpdateServer::default());

    {
        let update_server = update_server.clone();
        server::new(move || {
            let ws_state = ws_update::WsUpdateSessionState::new(update_server.clone());
            vec![
                App::with_state(ws_state)
                    .prefix("/ws")
                    .resource("/update/", |r| r.route().f(update_route))
                    .boxed(),
                App::with_state(ApiState {
                    db: db.clone()
                })
                    .prefix("/api")
                    .resource("/commands/", |r| r.method(http::Method::GET).f(commands_route))
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
