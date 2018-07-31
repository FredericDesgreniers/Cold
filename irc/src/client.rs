use actix::prelude::*;
use actix::Actor;
use actix::Addr;
use actix::Arbiter;
use actix::Context;
use actix::Handler;
use regex::Regex;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpStream;

lazy_static!{
    static ref MESSAGE_CHANNEL_REGEX: Regex= { // REGEX for normal channel messages
        Regex::new(r"^:(?P<user>.*)!.*@.*.tmi.twitch.tv PRIVMSG #(?P<channel>.*) :(?P<message>.*)\r\n$").unwrap()
    };
}

#[derive(Debug, Fail)]
pub enum IrcError {
    #[fail(display = "Connection failed: {}", 0)]
    ConnectionFailed(String),
    #[fail(display = "Could not read {}", 0)]
    ReadFailed(String),
    #[fail(display = "Could not write {}", 0)]
    WriteFailed(String),
}

/// Builds an irc client
/// Can specify auth parameters
pub struct IrcClientBuilder<'a> {
    url: &'a str,
    nickname: Option<&'a str>,
    password: Option<&'a str>,
}

impl<'a> IrcClientBuilder<'a> {
    /// Every irc client must have a url
    pub fn create(url: &'a str) -> Self {
        Self {
            url,
            nickname: None,
            password: None,
        }
    }

    /// Will send a nickname to the server once built.
    pub fn nick(mut self, nickname: &'a str) -> Self {
        self.nickname = Some(nickname);
        self
    }

    /// Will send a password to the server once built.
    pub fn pass(mut self, password: &'a str) -> Self {
        self.password = Some(password);
        self
    }

    /// Builds and connects and returns an irc client
    pub fn connect(self) -> Result<(IrcClientReader, Addr<IrcClientWriter>), IrcError> {
        let (reader, writer) = connect(self.url)?;

        if let Some(password) = self.password {
            writer.do_send(SendLine(format!("PASS {}", password)));
        }

        if let Some(nickname) = self.nickname {
            writer.do_send(SendLine(format!("NICK {}", nickname)));
        }

        Ok((reader, writer))
    }
}

/// Connect to the irc client using a url
/// `url` must be in the form: ip:port
pub fn connect(url: &str) -> Result<(IrcClientReader, Addr<IrcClientWriter>), IrcError> {
    let stream = match TcpStream::connect(url) {
        Ok(stream) => stream,
        Err(err) => return Err(IrcError::ConnectionFailed(format!("{:?}", err).to_string())),
    };

    let reader = IrcClientReader {
        reader: BufReader::new(
            stream
                .try_clone()
                .map_err(|_e| IrcError::ConnectionFailed("Cloning stream".to_owned()))?,
        ),
    };

    let writer = Arbiter::start(|_| IrcClientWriter {
        writer: BufWriter::new(stream),
    });

    Ok((reader, writer))
}

/// IRC channel that is received
#[derive(Debug)]
pub enum IrcMessage {
    ChannelMessage(ChannelMessage),
    Unknown(String),
}

/// A message from a specific channel
#[derive(Debug)]
pub struct ChannelMessage {
    pub user: String,
    pub channel: String,
    pub message: String,
}

pub struct IrcClientReader {
    reader: BufReader<TcpStream>,
}

impl IrcClientReader {
    /// Waits for a line from the irc server and returns it
    pub fn wait_for_line(&mut self) -> Result<String, IrcError> {
        let mut line = String::new();

        self.reader
            .read_line(&mut line)
            .map_err(|e| IrcError::ReadFailed(format!("{:?}", e)))?;

        Ok(line)
    }

    /// Get the next formatted message the irc
    pub fn next_message(&mut self) -> Result<IrcMessage, IrcError> {
        let line = self.wait_for_line()?;

        if let Some(captures) = MESSAGE_CHANNEL_REGEX.captures(&line) {
            return Ok(IrcMessage::ChannelMessage(ChannelMessage {
                user: captures["user"].to_owned(),
                channel: captures["channel"].to_owned(),
                message: captures["message"].to_owned(),
            }));
        }

        Ok(IrcMessage::Unknown(line))
    }
}

/// Actor that allows writing to an irc server
pub struct IrcClientWriter {
    writer: BufWriter<TcpStream>,
}

impl IrcClientWriter {
    /// Send a line through to the irc server
    /// This will append a \n to the message
    pub fn send_line(&mut self, line: &str) -> Result<(), IrcError> {
        let _ = self
            .writer
            .write(line.as_bytes())
            .map_err(|e| IrcError::WriteFailed(format!("{:?}", e)))?;
        self.send_new_line()?;
        self.writer
            .flush()
            .map_err(|err| IrcError::WriteFailed(format!("{:?}", err)))?;
        Ok(())
    }

    /// Send `\n` to the irc server
    pub fn send_new_line(&mut self) -> Result<(), IrcError> {
        let _ = self
            .writer
            .write(b"\n")
            .map_err(|e| IrcError::WriteFailed(format!("{:?}", e)))?;
        Ok(())
    }

    /// Join an irc channel
    /// This will send JOIN #`channel_name`
    pub fn join(&mut self, channel_name: &str) -> Result<(), IrcError> {
        self.send_line(&format!("JOIN #{}", channel_name))
    }
}

impl Actor for IrcClientWriter {
    type Context = Context<Self>;
}

/// Send a raw line to irc server
pub struct SendLine(pub String);

impl Message for SendLine {
    type Result = Result<(), IrcError>;
}

impl Handler<SendLine> for IrcClientWriter {
    type Result = Result<(), IrcError>;

    fn handle(
        &mut self,
        msg: SendLine,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<SendLine>>::Result {
        self.send_line(&msg.0)?;
        Ok(())
    }
}

/// Join an irc channel
pub struct JoinChannel(pub String);

impl Message for JoinChannel {
    type Result = Result<(), IrcError>;
}

impl Handler<JoinChannel> for IrcClientWriter {
    type Result = Result<(), IrcError>;

    fn handle(
        &mut self,
        msg: JoinChannel,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<JoinChannel>>::Result {
        self.join(&msg.0)
    }
}

/// Send a message to an IRC channel
pub struct SendChannelMessage {
    pub channel: String,
    pub message: String,
}

impl Message for SendChannelMessage {
    type Result = Result<(), IrcError>;
}

impl Handler<SendChannelMessage> for IrcClientWriter {
    type Result = Result<(), IrcError>;

    fn handle(
        &mut self,
        msg: SendChannelMessage,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<SendChannelMessage>>::Result {
        self.send_line(&format!("PRIVMSG #{} :{}", msg.channel, msg.message))
    }
}
