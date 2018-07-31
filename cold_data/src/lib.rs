#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derives;
extern crate actix;
extern crate dotenv;
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use actix::{Actor, Addr, Handler, SyncArbiter, SyncContext};
use diesel::mysql::MysqlConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use failure::Error;
use std::env;

pub mod models;
mod schema;
use schema::commands::dsl::*;

/// A database connection pool in order to properly utilize the actor system
pub struct DbConnectionPool {
    connection: Pool<ConnectionManager<MysqlConnection>>,
}

impl DbConnectionPool {
    /// Connect to database and establish a connection pool
    pub fn connect() -> Addr<DbConnectionPool> {
        dotenv::dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("Database url not set");

        let connection = ConnectionManager::<MysqlConnection>::new(database_url);

        let pool = Pool::builder()
            .build(connection)
            .expect("Failed to crate db pool");

        SyncArbiter::start(3, move || Self {
            connection: pool.clone(),
        })
    }
}

impl Actor for DbConnectionPool {
    type Context = SyncContext<Self>;
}

impl Handler<models::CreateCommand> for DbConnectionPool {
    type Result = Result<usize, Error>;

    fn handle(
        &mut self,
        msg: models::CreateCommand,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<models::CreateCommand>>::Result {
        println!("{:?}", msg.match_expr);

        let connection = self.connection.get()?;

        Ok(diesel::replace_into(schema::commands::table)
            .values(&vec![(
                channel.eq(msg.channel),
                match_expr.eq(msg.match_expr),
                command.eq(msg.command),
            )])
            .execute(&connection)?)
    }
}
