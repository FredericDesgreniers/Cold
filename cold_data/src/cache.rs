use actix::{Actor, Addr};
use diesel::prelude::*;
use failure::Error;
use futures::Future;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::RwLock;

use super::{models::Command, schema::commands::dsl::*, DbConnectionPool, ListCommands};
use std::collections::HashMap;

#[derive(Debug, Fail)]
pub enum CacheError {
    #[fail(display = "Could not acquire writer {}", 0)]
    WriterError(String),
}

#[derive(Clone)]
pub struct CommandCache {
    inner: Arc<RwLock<CommandCacheInner>>,
}

impl CommandCache {
    pub fn new() -> Self {
        CommandCache {
            inner: Arc::new(RwLock::new(CommandCacheInner {
                commands: Vec::new(),
            })),
        }
    }

    pub fn update_async<'a>(
        &'a self,
        db: Addr<DbConnectionPool>,
    ) -> impl Future<Item = (), Error = Error> + 'a {
        let cache_clone = self.clone();

        db.send(ListCommands {}).then(move |result| {
            let mut writer = cache_clone
                .write()
                .map_err(|err| CacheError::WriterError(format!("{:?}", err)))?;
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

        let result = commands.load::<Command>(&connection)?;

        let mut writer = self
            .write()
            .map_err(|err| CacheError::WriterError(format!("{:?}", err)))?;

        writer.commands = result;

        Ok(())
    }
}

impl Deref for CommandCache {
    type Target = Arc<RwLock<CommandCacheInner>>;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.inner
    }
}

pub struct CommandCacheInner {
    pub commands: Vec<Command>,
}
