use robotech::macros::svc;

#[svc(after_write = crate::cache::on_queue_cache_refresh)]
pub struct MsgMessageQueueSvc;