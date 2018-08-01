extern crate actix;
extern crate cold_data;
extern crate futures;
extern crate irc;
extern crate web_frontend;
extern crate serde;
extern crate serde_json;

use actix::SyncArbiter;
use actix::SyncContext;
use actix::{Actor, Addr, Arbiter, Context, Handler, Message};
use cold_data::cache::CommandCache;
use cold_data::models::Command;
use cold_data::DbConnectionPool;
use futures::Future;
use irc::client::IrcClientWriter;
use web_frontend::ws_update::UpdateServer;
use web_frontend::ws_update::MassSend;

/// Actor that processes various test commands
pub struct CommandProcessor {
    db: Addr<DbConnectionPool>,
    irc_writer: Addr<IrcClientWriter>,
    update_server: Addr<UpdateServer>,
    commands: CommandCache,
}

impl CommandProcessor {
    pub fn create(
        db: Addr<DbConnectionPool>,
        irc_writer: Addr<IrcClientWriter>,
        update_server: Addr<UpdateServer>,
        commands: CommandCache,
    ) -> Addr<Self> {
        SyncArbiter::start(3, move || Self {
            db: db.clone(),
            irc_writer: irc_writer.clone(),
            commands: commands.clone(),
            update_server: update_server.clone(),
        })
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
                "remove" => {
                    let match_expr = rest.split(' ').nth(0);
                    if let Some(match_expr) = match_expr {
                        let result = self.db.send(cold_data::models::RemoveCommand {
                            channel: channel.clone(),
                            match_expr: match_expr.to_owned(),
                        })
                                         .from_err()
                                         .and_then(|result| {
                                             match result {
                                                 Ok(res) => {
                                                     if res > 0 {
                                                         let commands = self.commands.read().expect("READ ERROR");
                                                         let json_commands = serde_json::to_string(&commands.commands)?;
                                                         self.update_server.do_send(MassSend { message: json_commands });

                                                         self.irc_writer.do_send(irc::client::SendChannelMessage {
                                                             channel,
                                                             message: format!("@{} Command has been removed!", user),
                                                         });
                                                     }
                                                     Ok(res)
                                                 }
                                                 Err(err) => {
                                                     println!("Error with command {:?}", err);
                                                     self.irc_writer.do_send(irc::client::SendChannelMessage {
                                                         channel,
                                                         message: format!("@{} Command could not be removed, does it exist?", user),
                                                     });
                                                     Err(err)
                                                 }
                                             }
                                         })
                                         .wait();
                    }
                }
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
                                    let commands = self.commands.read().expect("READ ERROR");
                                    let json_commands = serde_json::to_string(&commands.commands)?;
                                    self.update_server.do_send(MassSend{message: json_commands});

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
