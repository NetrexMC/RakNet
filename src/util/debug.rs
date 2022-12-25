/// A wrapper for println that is enabled
/// only when debug is enabled.
#[macro_export]
macro_rules! rakrs_debug {
    ($heavy: ident, $($t: tt)*) => {
        if cfg!(feature="debug") && cfg!(feature="debug-all") {
            println!("[rakrs] DBG! {}", format!($($t)*));
        }
    };
    ($($t: tt)*) => {
        if cfg!(feature="debug") {
            println!("[rakrs] DBG! {}", format!($($t)*));
        }
    };
}