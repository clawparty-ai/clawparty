#[macro_export]
macro_rules! ts_print {
    ($($arg:tt)*) => {
        println!("{} {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! ts_eprint {
    ($($arg:tt)*) => {
        eprintln!("{} {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), format!($($arg)*))
    };
}
