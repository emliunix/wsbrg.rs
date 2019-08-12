use crate::wsproto::frame::Frame;

trait Handler {
    fn on_start();
    fn on_message();
    fn on_error();
    fn on_close();
}

pub enum SessionState {
    Connecting,
    Open,
    Close,
}

struct Session<H> where H: Handler {
    state: SessionState,
    handler: H,
}

impl<H> Session<H> where H: Handler {
    pub fn new(h: H) -> Self {
        Session {
            state: SessionState::Connecting,
            handler: h,
        }
    }
    pub fn on_frame(frame: Frame) {

    }
}