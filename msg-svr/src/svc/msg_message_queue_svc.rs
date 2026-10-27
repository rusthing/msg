use robotech::macros::svc;

#[svc(after_write = crate::sub::on_message_queue_refresh)]
pub struct MsgMessageQueueSvc;