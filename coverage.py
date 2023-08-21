import os
os.system("CARGO_INCREMENTAL=0 RUSTFLAGS='-Cinstrument-coverage' LLVM_PROFILE_FILE='cargo-test-%p-%m.profraw' cargo test")
os.system("gcov . --binary-path ./target/debug/deps -s . -t html --branch --ignore-not-existing --ignore ../* --ignore /* --ignore xtask/* --ignore */src/tests/* -o coverage/html")