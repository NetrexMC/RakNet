/// A wrapper for println that is enabled
/// only when debug is enabled.
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
