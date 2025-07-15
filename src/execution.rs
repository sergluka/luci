use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};

use serde_json::Value;
use slotmap::{new_key_type, SlotMap};

use crate::{
    messages::Messages,
    names::{ActorName, EventName},
    scenario::{Msg, RequiredToBe},
};

mod build;
mod report;
mod runner;

pub use build::BuildError;
pub use build::ExecutableBuilder;
pub use report::Report;
pub use runner::RunError;
pub use runner::Running;

new_key_type! {
    pub struct KeyBind;
    pub struct KeySend;
    pub struct KeyRecv;
    pub struct KeyRespond;
    pub struct KeyDelay;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EventKey {
    Bind(KeyBind),
    Send(KeySend),
    Recv(KeyRecv),
    Respond(KeyRespond),
    Delay(KeyDelay),
}

#[derive(Debug)]
pub struct Executable {
    messages: Messages,
    events: Events,
}

#[derive(Debug, Default)]
struct Events {
    priority: HashMap<EventKey, usize>,
    required: HashMap<EventKey, RequiredToBe>,
    names: HashMap<EventKey, EventName>,

    bind: SlotMap<KeyBind, VertexBind>,
    send: SlotMap<KeySend, VertexSend>,
    recv: SlotMap<KeyRecv, VertexRecv>,
    respond: SlotMap<KeyRespond, VertexRespond>,
    delay: SlotMap<KeyDelay, VertexDelay>,

    entry_points: BTreeSet<EventKey>,

    key_unblocks_values: HashMap<EventKey, BTreeSet<EventKey>>,
}

#[derive(Debug)]
struct VertexSend {
    from: ActorName,
    to: Option<ActorName>,
    fqn: Arc<str>,
    payload: Msg,
}

#[derive(Debug)]
struct VertexRecv {
    from: Option<ActorName>,
    to: Option<ActorName>,
    fqn: Arc<str>,
    payload: Msg,
}

#[derive(Debug)]
struct VertexRespond {
    respond_to: KeyRecv,
    request_type: Arc<str>,
    respond_from: Option<ActorName>,
    payload: Msg,
}

#[derive(Debug)]
struct VertexBind {
    dst: Value,
    src: Msg,
}

#[derive(Debug)]
struct VertexDelay {
    delay_for: Duration,
    delay_step: Duration,
}
