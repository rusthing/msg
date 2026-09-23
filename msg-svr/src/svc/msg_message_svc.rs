use robotech::macros::svc;

#[svc(after_write = super::refresh_msg_message_cache)]
pub struct MsgMessageSvc;