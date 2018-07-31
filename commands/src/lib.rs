extern crate actix;
extern crate cold_data;
extern crate futures;
extern crate irc;

use actix::{Actor, Addr, Arbiter, Context, Handler, Message};
use cold_data::DbConnectionPool;
use futures::Future;
use irc::client::IrcClientWriter;

/// Actor that processes various test commands
pub struct CommandProcessor {
    db: Addr<DbConnectionPool>,
    irc_writer: Addr<IrcClientWriter>,
}

impl CommandProcessor {
    pub fn create(db: Addr<DbConnectionPool>, irc_writer: Addr<IrcClientWriter>) -> Addr<Self> {
        Arbiter::start(|_| Self { db, irc_writer })
    }
}

impl Actor for CommandProcessor {
    type Context = Context<Self>;
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
                                "@{} set command should be in the form: \"#set match command\"!",
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
