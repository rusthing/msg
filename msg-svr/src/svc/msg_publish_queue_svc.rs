use robotech::macros::svc;

#[svc(after_write = crate::sub::on_publish_queue_refresh)]
pub struct MsgPublishQueueSvc;