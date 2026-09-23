use robotech::macros::svc;

#[svc(after_write = super::refresh_msg_queue_cache)]
pub struct MsgMessageQueueSvc;