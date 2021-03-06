use super::schema::commands;
use actix::prelude::*;
use failure::Error;

/// Create a bot command in the database
#[derive(PartialEq, Eq, Insertable)]
#[table_name = "commands"]
pub struct CreateCommand {
    pub channel: String,
    pub match_expr: String,
    pub command: String,
}

impl Message for CreateCommand {
    type Result = Result<usize, Error>;
}

pub struct RemoveCommand {
    pub channel: String,
    pub match_expr: String,
}

impl Message for RemoveCommand {
    type Result = Result<usize, Error>;
}

#[derive(Serialize, Queryable)]
pub struct Command {
    pub channel: String,
    pub match_expr: String,
    pub command: String,
}


pub struct ListCommands {}

impl Message for ListCommands {
    type Result = Result<Vec<Command>, Error>;
}

