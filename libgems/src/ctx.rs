/// Describes a context
pub trait AppCtx {
    type State;
    type Message;

    fn send_message(&mut self, msg: &Self::Message);
    fn state_mut(&mut self) -> &mut Self::State;
}

impl AppCtx for () {
    type Message = ();
    type State = ();
    fn send_message(&mut self, msg: &Self::Message) {
        _ = msg;
    }
    fn state_mut(&mut self) -> &mut Self::State {
        self
    }
}
