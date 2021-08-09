pub fn is_ack_or_nack(byte: u8) -> bool {
     byte == 0xa0 || byte == 0xc0
}