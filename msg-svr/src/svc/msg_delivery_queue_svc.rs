use robotech::macros::svc;

#[svc(after_write = crate::wkr::on_delivery_queue_refresh)]
pub struct MsgDeliveryQueueSvc;
