/// A wrapper for println that is enabled only with the features `debug`, `debug_all`, or `debug_features`.
/// - `debug` - Enables generic logging, purposeful for debugging things going wrong with your implementation
/// - `debug_all` - Enables logging of all components, IE, connection, packets, etc.
/// - `debug_buffers` - Enables logging of all buffers sent and received.
#[macro_export]
macro_rules! rakrs_debug {
    ($heavy: ident, $($t: tt)*) => {
        if cfg!(feature="debug") && cfg!(feature="debug_all") {
            println!("[rakrs] DBG! {}", format!($($t)*));
        }
    };
    ($($t: tt)*) => {
        if cfg!(feature="debug") {
            println!("[rakrs] DBG! {}", format!($($t)*));
        }
    };
}

#[macro_export]
macro_rules! rakrs_debug_buffers {
    ($server: literal, $($t: tt)*) => {
        if cfg!(feature="debug_buffers") {
            let x = if $server == true { "S -> C" } else { "C -> S" };
            println!("[rakrs] DBG [{}]: {}", x, format!($($t)*));
        }
    };
}
