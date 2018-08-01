extern crate actix;
extern crate cold_data;
extern crate futures;
extern crate irc;

use actix::{Actor, Addr, Arbiter, Context, Handler, Message};
use cold_data::DbConnectionPool;
use futures::Future;
use irc::client::IrcClientWriter;
use actix::SyncArbiter;
use actix::SyncContext;
use cold_data::models::Command;
use std::ops::Deref;
use cold_data::cache::CommandCache;


/// Actor that processes various test commands
pub struct CommandProcessor {
    db: Addr<DbConnectionPool>,
    irc_writer: Addr<IrcClientWriter>,
    commands: CommandCache
}

impl CommandProcessor {
    pub fn create(db: Addr<DbConnectionPool>, irc_writer: Addr<IrcClientWriter>, commands: CommandCache) -> Addr<Self> {
        SyncArbiter::start(3, move || Self { db: db.clone(), irc_writer: irc_writer.clone(), commands: commands.clone() })
    }
}

impl Actor for CommandProcessor {
    type Context = SyncContext<Self>;
}

pub struct MetaCommand {
    pub channel: String,
    pub user: String,
    pub message: String,
}

impl Message for MetaCommand {
    type Result = String;
}

impl Handler<MetaCommand> for CommandProcessor {
    type Result = String;

    fn handle(
        &mut self,
        msg: MetaCommand,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<MetaCommand>>::Result {
        let MetaCommand {
            channel,
            user,
            message,
        } = msg;


        if let Some(index) = message.find(' ') {
            let (command, rest) = message.split_at(index);
            let rest = rest.trim();

            match command {
                "set" => {
                    let key_index = rest.find(' ');

                    if let Some(key_index) = key_index {
                        let (keyword, rest) = rest.split_at(key_index);

                        let _ = self
                            .db
                            .send(cold_data::models::CreateCommand {
                                channel: channel.clone(),
                                match_expr: keyword.to_owned(),
                                command: rest.to_owned(),
                            })
                            .from_err()
                            .and_then(|result| match result {
                                Ok(res) => {
                                    self.irc_writer.do_send(irc::client::SendChannelMessage {
                                        channel,
                                        message: format!("@{} Command has been set!", user),
                                    });
                                    Ok(res)
                                }
                                Err(err) => {
                                    println!("Error with command {:?}", err);
                                    self.irc_writer.do_send(irc::client::SendChannelMessage {
												  channel,
												  message: format!("@{} Command could not be set, ask the bot owner to check logs!", user),
											  });
                                    Err(err)
                                }
                            })
                            .wait();
                    } else {
                        self.irc_writer.do_send(irc::client::SendChannelMessage {
                            channel,
                            message: format!(
                                "@{} set command should be in the form: \"#set match_expression command\"!",
                                user
                            ),
                        });
                    }
                }
                _ => {}
            }
        }

        String::new()
    }
}
