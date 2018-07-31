extern crate irc;
extern crate failure;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate web_frontend;
extern crate dotenv;
extern crate cold_data;
extern crate actix;
extern crate futures;

mod config;

use failure::Error;
use irc::client::{IrcClientBuilder, IrcMessage};
use std::thread;
use dotenv::dotenv;
use cold_data::DbConnectionPool;
use futures::prelude::*;
use std::sync::Arc;
use actix::Addr;
use web_frontend::start_server;
use web_frontend::ws_update::UpdateServer;

fn main() -> Result<(), Error> {

	let system = actix::System::new("Some system");

	dotenv().ok();
	let config = Arc::new(config::load_config_toml("config.toml").expect("Could not load config"));

	let db = DbConnectionPool::connect();

	let update_server =	start_server();

	{
		let config = config.clone();
		let db = db.clone();
		let update_server = update_server.clone();

		run_irc(db, config, update_server).unwrap();
	}

	system.run();

	Ok(())
}

//NOTE: This needs to be moved out and re-structured.
fn run_irc(db: Addr<DbConnectionPool>, config: Arc<config::Config>, update_server: Addr<UpdateServer>) -> Result<(), Error> {
	let (mut reader, writer) = IrcClientBuilder::create(&config.twitch.irc_server)
		.nick(&config.twitch.username)
		.pass(&config.twitch.token)
		.connect()?;

	// We spawn the thread after making hte irc client since IrcClientBuilder will create an actor that needs to be on the main thread
	thread::spawn(move ||{
		for channel in &config.twitch.channels {
			writer.do_send(irc::client::JoinChannel(channel.clone()));
		}

		loop {
			match reader.next_message().expect("Message could not be received") {
				IrcMessage::ChannelMessage(message) => {
					println!("{:?}", message);
					if message.message.starts_with("#") {
						let channel = message.channel;
						let user = message.user;
						let text = message.message[1..].trim();

						if let Some(index) = text.find(' ') {
							let (command, rest) = text.split_at(index);

							let rest = rest.trim();

							match command {
								"set" => {
									let key_index = rest.find(' ');

									if let Some(key_index) = key_index {
										let (keyword, rest) = rest.split_at(key_index);

										let _ = db.send(cold_data::models::CreateCommand{
											channel: channel.clone(),
											match_expr: keyword.to_owned(),
											command: rest.to_owned(),
										})
												  .from_err()
												  .and_then(|result| {
													  match result {
														  Ok(res) => {
															  writer.do_send(irc::client::SendChannelMessage {
																  channel,
																  message: format!("@{} Command has been set!", user),
															  });
															  Ok(res)
														  },
														  Err(err) => {
															  println!("Error with command {:?}", err);
															  writer.do_send(irc::client::SendChannelMessage {
																  channel,
																  message: format!("@{} Command could not be set, ask the bot owner to check logs!", user),
															  });
															  Err(err)
														  }
													  }
												  })
												  .wait();
									} else {
										writer.do_send(irc::client::SendChannelMessage {
											channel,
											message: format!("@{} set command should be in the form: \"#set match command\"!", user),
										});
									}
								}
								_ => {}
							}
						}
					} else {
						update_server.do_send(web_frontend::ws_update::MassSend{message: format!("{:?}", message)});
					}
				},
				_ => {

				}
			}
		}
	});

	Ok(())
}

