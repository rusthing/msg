use robotech::macros::svc;

#[svc(after_write = crate::cache::on_message_cache_refresh)]
pub struct MsgChannelSvc;