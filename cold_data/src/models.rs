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

#[derive(Serialize, Queryable)]
pub struct Command {
    pub channel: String,
    pub match_expr: String,
    pub command: String,
}

impl<'a> Message for CreateCommand {
    type Result = Result<usize, Error>;
}
