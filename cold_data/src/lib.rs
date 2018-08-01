#![feature(attr_literals)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derives;
extern crate actix;
extern crate dotenv;
#[macro_use]
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate futures;

use actix::{Actor, Addr, Handler, SyncArbiter, SyncContext};
use diesel::mysql::MysqlConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use failure::Error;
use futures::Future;
use std::env;

pub mod cache;
pub mod models;

mod schema;
use cache::CommandCache;
use models::ListCommands;
use schema::commands::dsl::*;

/// A database connection pool in order to properly utilize the actor system
pub struct DbConnectionPool {
    connection: Pool<ConnectionManager<MysqlConnection>>,
    command_cache: CommandCache,
}

impl DbConnectionPool {
    /// Connect to database and establish a connection pool
    pub fn connect(command_cache: CommandCache) -> Addr<DbConnectionPool> {
        //This would eventually become a builder if more than one cache is needed
        dotenv::dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("Database url not set");

        let connection = ConnectionManager::<MysqlConnection>::new(database_url);

        let pool = Pool::builder()
            .build(connection)
            .expect("Failed to crate db pool");

        SyncArbiter::start(3, move || Self {
            connection: pool.clone(),
            command_cache: command_cache.clone(),
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

        let row_change = diesel::replace_into(commands)
            .values(&vec![(
                channel.eq(msg.channel),
                match_expr.eq(msg.match_expr),
                command.eq(msg.command),
            )])
            .execute(&connection)?;

        if row_change > 0 {
            self.command_cache.update(self)?;
        }

        Ok(row_change)
    }
}

impl Handler<models::RemoveCommand> for DbConnectionPool {
    type Result = Result<usize, Error>;

    fn handle(&mut self, msg: models::RemoveCommand, _ctx: &mut Self::Context) -> <Self as Handler<models::RemoveCommand>>::Result {
        let connection = self.connection.get()?;

        let row_change = diesel::delete(commands.filter((channel.eq(msg.channel).and(match_expr.eq(msg.match_expr)))))
            .execute(&connection).map_err(|err| Error::from(err))?;

        if row_change > 0 {
            self.command_cache.update(self)?;
        }

        Ok(row_change)
    }
}

impl Handler<ListCommands> for DbConnectionPool {
    type Result = Result<Vec<models::Command>, Error>;

    fn handle(
        &mut self,
        msg: ListCommands,
        ctx: &mut Self::Context,
    ) -> <Self as Handler<ListCommands>>::Result {
        let connection = self.connection.get()?;

        let result = commands.load::<models::Command>(&connection)?;

        Ok(result)
    }
}
