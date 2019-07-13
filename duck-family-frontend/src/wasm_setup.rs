#[allow(unused_macros)]
#[cfg(target_arch = "wasm32")]
macro_rules! println {
    ($($tt:tt)*) => {{
        let msg = format!($($tt)*);
        js! { console.log(@{ msg }) }
    }}
}
#[allow(unused_macros)]
#[cfg(target_arch = "wasm32")]
macro_rules! error {
    ($($tt:tt)*) => {{
        let msg = format!($($tt)*);
        js! { console.error(@{ msg }) }
    }}
}

#[cfg(target_arch = "wasm32")]
pub fn setup_wasm() {
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {}\n", &panic_info);
    }));
    stdweb::initialize();
    // stdweb::event_loop();
}

use crate::stdweb::unstable::TryInto;
pub fn local_now() -> f64 {
    js!(
        var date = new Date();
        return date.getTime() - date.getTimezoneOffset()*60*1000;
    ).try_into().unwrap()
}