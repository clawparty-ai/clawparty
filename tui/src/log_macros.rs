#[macro_export]
macro_rules! ts_print {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            let trimmed = msg.trim_start();
            // Detect if msg already starts with a timestamp like "YYYY-MM-DD HH:MM:SS.mmm"
            let has_ts = trimmed.len() >= 23
                && trimmed.as_bytes()[4] == b'-'
                && trimmed.as_bytes()[7] == b'-'
                && trimmed.as_bytes()[10] == b' '
                && trimmed.as_bytes()[13] == b':'
                && trimmed.as_bytes()[16] == b':'
                && trimmed.as_bytes()[19] == b'.';
            if has_ts {
                println!("{}", msg);
            } else {
                println!("{} {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), msg);
            }
        }
    };
}

#[macro_export]
macro_rules! ts_eprint {
    ($($arg:tt)*) => {
        {
            let msg = format!($($arg)*);
            let trimmed = msg.trim_start();
            let has_ts = trimmed.len() >= 23
                && trimmed.as_bytes()[4] == b'-'
                && trimmed.as_bytes()[7] == b'-'
                && trimmed.as_bytes()[10] == b' '
                && trimmed.as_bytes()[13] == b':'
                && trimmed.as_bytes()[16] == b':'
                && trimmed.as_bytes()[19] == b'.';
            if has_ts {
                eprintln!("{}", msg);
            } else {
                eprintln!("{} {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), msg);
            }
        }
    };
}
