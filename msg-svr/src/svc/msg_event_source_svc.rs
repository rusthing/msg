use robotech::macros::svc;

#[svc(after_write = crate::cache::on_cache_refresh)]
pub struct MsgEventSourceSvc;