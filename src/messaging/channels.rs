// Communication channels lock-free

use crate::messaging::command::Command;
use crate::messaging::notification::Notification;
use ringbuf::{HeapRb, traits::Split};

pub type CommandProducer = ringbuf::HeapProd<Command>;
pub type CommandConsumer = ringbuf::HeapCons<Command>;

pub fn create_command_channel(capacity: usize) -> (CommandProducer, CommandConsumer) {
    let rb = HeapRb::<Command>::new(capacity);
    rb.split()
}

pub type NotificationProducer = ringbuf::HeapProd<Notification>;
pub type NotificationConsumer = ringbuf::HeapCons<Notification>;

pub fn create_notification_channel(
    capacity: usize,
) -> (NotificationProducer, NotificationConsumer) {
    let rb = HeapRb::<Notification>::new(capacity);
    rb.split()
}
