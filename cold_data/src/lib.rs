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

use futures::Future;
use actix::{Actor, Addr, Handler, SyncArbiter, SyncContext};
use diesel::mysql::MysqlConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use failure::Error;
use std::env;

pub mod models;
mod schema;
use schema::commands::dsl::*;
use models::ListCommands;
use std::sync::Arc;
use std::sync::RwLock;
use std::ops::Deref;
use models::Command;

/// A database connection pool in order to properly utilize the actor system
pub struct DbConnectionPool {
    connection: Pool<ConnectionManager<MysqlConnection>>,
	command_cache: CommandCache
}

impl DbConnectionPool {
    /// Connect to database and establish a connection pool
    pub fn connect(command_cache: CommandCache) -> Addr<DbConnectionPool> { //This would eventually become a builder if more than one cache is needed
        dotenv::dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("Database url not set");

        let connection = ConnectionManager::<MysqlConnection>::new(database_url);

        let pool = Pool::builder()
            .build(connection)
            .expect("Failed to crate db pool");

        SyncArbiter::start(3, move || Self {
            connection: pool.clone(),
			command_cache: command_cache.clone()
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

        let row_change = diesel::replace_into(schema::commands::table)
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

impl Handler<ListCommands> for DbConnectionPool {
    type Result = Result<Vec<models::Command>, Error>;

    fn handle(&mut self, msg: ListCommands, ctx: &mut Self::Context) -> <Self as Handler<ListCommands>>::Result {
        let connection = self.connection.get()?;

        let result = commands.load::<models::Command>(&connection)?;

        Ok(result)
    }
}

#[derive(Debug, Fail)]
pub enum CacheError {
	#[fail(display="Could not acquire writer {}", 0)]
	WriterError(String)
}

#[derive(Clone)]
pub struct CommandCache(Arc<RwLock<CommandCacheInner>>);

impl CommandCache {
	pub fn new() -> Self {
		CommandCache(Arc::new(RwLock::new(CommandCacheInner {
			commands: Vec::new()
		})))
	}

	pub fn update_async<'a>(&'a self, db: Addr<DbConnectionPool>) -> impl Future<Item = (), Error = Error> + 'a{
		let cache_clone = self.clone();

		db.send(ListCommands{})
			.then(move |result| {
				let mut writer = cache_clone.write().map_err(|err| CacheError::WriterError(format!("{:?}", err)))?;
				if let Ok(result) = result {
					if let Ok(result) = result {
						writer.commands = result;
					}
				}

				Ok(())
			})
	}

	pub fn update(&self, db: &DbConnectionPool) -> Result<(), Error> {
		let connection = db.connection.get()?;

		let result = commands.load::<models::Command>(&connection)?;

		let mut writer = self.write().map_err(|err| CacheError::WriterError(format!("{:?}", err)))?;

		writer.commands = result;

		Ok(())
	}
}

impl Deref for CommandCache {
	type Target = Arc<RwLock<CommandCacheInner>>;

	fn deref(&self) -> &<Self as Deref>::Target {
		&self.0
	}
}

pub struct CommandCacheInner {
	commands: Vec<Command>
}
