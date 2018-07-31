extern crate failure;
extern crate irc;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate actix;
extern crate cold_data;
extern crate commands;
extern crate dotenv;
extern crate futures;
extern crate toml;
extern crate web_frontend;

mod config;

use actix::Addr;
use cold_data::DbConnectionPool;
use commands::CommandProcessor;
use dotenv::dotenv;
use failure::Error;
use irc::client::IrcClientReader;
use irc::client::IrcClientWriter;
use irc::client::{IrcClientBuilder, IrcMessage};
use std::sync::Arc;
use std::thread;
use web_frontend::start_server;
use web_frontend::ws_update::UpdateServer;

fn main() -> Result<(), Error> {
    let system = actix::System::new("Some system");

    dotenv().ok();
    let config = Arc::new(config::load_config_toml("config.toml").expect("Could not load config"));

    let db = DbConnectionPool::connect();

    let update_server = start_server();

    let (reader, writer) = IrcClientBuilder::create(&config.twitch.irc_server)
        .nick(&config.twitch.username)
        .pass(&config.twitch.token)
        .connect()?;

    let command_processor = CommandProcessor::create(db.clone(), writer.clone());

    {
        let config = config.clone();
        let update_server = update_server.clone();

        run_irc(reader, writer, command_processor, config, update_server).unwrap();
    }

    system.run();

    Ok(())
}

//NOTE: This needs to be moved out and re-structured.
fn run_irc(
    mut reader: IrcClientReader,
    writer: Addr<IrcClientWriter>,
    command_processor: Addr<CommandProcessor>,
    config: Arc<config::Config>,
    update_server: Addr<UpdateServer>,
) -> Result<(), Error> {
    // We spawn the thread after making hte irc client since IrcClientBuilder will create an actor that needs to be on the main thread
    thread::spawn(move || {
        for channel in &config.twitch.channels {
            writer.do_send(irc::client::JoinChannel(channel.clone()));
        }

        loop {
            match reader
                .next_message()
                .expect("Message could not be received")
            {
                IrcMessage::ChannelMessage(message) => {
                    println!("{:?}", message);
                    if message.message.starts_with("#") {
                        let channel = message.channel;
                        let user = message.user;
                        let text = message.message[1..].trim();

                        command_processor.do_send(commands::MetaCommand {
                            channel,
                            user,
                            message: text.to_owned(),
                        });
                    } else {
                        update_server.do_send(web_frontend::ws_update::MassSend {
                            message: format!("{:?}", message),
                        });
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}