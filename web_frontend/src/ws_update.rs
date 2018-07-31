use actix::fut;
use actix::prelude::*;
use actix_web::ws;
use rand;
use rand::{Rng, ThreadRng};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

/// A message that is sent through web socket
#[derive(Message)]
pub struct Message(pub String);

/// Server responsible for updating the frontend
pub struct UpdateServer {
    clients: HashMap<usize, Recipient<Message>>,
    rng: RefCell<ThreadRng>,
}

impl Default for UpdateServer {
    fn default() -> Self {
        Self {
            clients: Default::default(),
            rng: RefCell::new(rand::thread_rng()), //This is statistically unique, however, something more robust should be used
        }
    }
}

impl UpdateServer {
    /// Send message to all connected frontend clients
    pub fn send_update(&self, message: &str) {
        self.clients.iter().for_each(|(_id, client)| {
            let _ = client.do_send(Message(message.to_owned()));
        });
    }
}

impl Actor for UpdateServer {
    type Context = Context<Self>;
}

/// Message sent when client connects to update server
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
}

impl Handler<Connect> for UpdateServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> usize {
        let id = self.rng.borrow_mut().gen::<usize>();
        self.clients.insert(id, msg.addr);
        println!("Connected: {}", id);

        id
    }
}

/// Message sent when clients disconnects from server
#[derive(Message)]
#[rtype(usize)]
pub struct Disconnect {
    pub id: usize,
}

impl Handler<Disconnect> for UpdateServer {
    type Result = usize;

    fn handle(&mut self, msg: Disconnect, _ctx: &mut Context<Self>) -> usize {
        self.clients.remove(&msg.id);
        println!("Disconnected: {}", msg.id);
        msg.id
    }
}

#[derive(Debug, Message)]
pub struct MassSend {
    pub message: String,
}

impl Handler<MassSend> for UpdateServer {
    type Result = ();

    fn handle(
        &mut self,
        msg: MassSend,
        _ctx: &mut Self::Context,
    ) -> <Self as Handler<MassSend>>::Result {
        for (_, client) in &self.clients {
            let _ = client.do_send(Message(msg.message.clone()));
        }
    }
}

pub struct WsUpdateSessionState {
    addr: Addr<UpdateServer>,
}

impl WsUpdateSessionState {
    pub fn new(addr: Addr<UpdateServer>) -> Self {
        Self { addr }
    }
}

pub struct WsUpdateSession {
    id: usize,
    hb: Instant,
}

impl Default for WsUpdateSession {
    fn default() -> Self {
        WsUpdateSession {
            id: 0,
            hb: Instant::now(),
        }
    }
}

impl Actor for WsUpdateSession {
    type Context = ws::WebsocketContext<Self, WsUpdateSessionState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();

        ctx.state()
            .addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }
                fut::ok(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        ctx.state().addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<Message> for WsUpdateSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsUpdateSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Pong(_) => self.hb = Instant::now(),
            ws::Message::Text(text) => {
                let m = text.trim();
                match m {
                    m => println!("Unrecognized Message: {}", m),
                }
            }
            ws::Message::Binary(_) => println!("Update server cannot handle binary"),
            ws::Message::Close(_) => {
                ctx.stop();
            }
        }
    }
}
