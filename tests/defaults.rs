use rakrs::{protocol::mcpe::motd::Motd, server::start};

#[test]
fn run_test() {
    let motd = Motd::new(12, 19132.to_string());
    assert_eq!(motd.port, 19132.to_string());
}
