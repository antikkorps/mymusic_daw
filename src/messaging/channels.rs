// Communication channels lock-free

use crate::messaging::command::Command;
use ringbuf::{HeapRb, traits::Split};

pub type CommandProducer = ringbuf::HeapProd<Command>;
pub type CommandConsumer = ringbuf::HeapCons<Command>;

pub fn create_command_channel(capacity: usize) -> (CommandProducer, CommandConsumer) {
    let rb = HeapRb::<Command>::new(capacity);
    rb.split()
}
