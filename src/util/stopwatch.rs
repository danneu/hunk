// let x = stopwatch2! {
//     slow_calculation();
//     slow_calculation();
//     42
// };
// assert_eq!(x, 42);
//
// let x = stopwatch2!("test", {
//     slow_calculation();
//     slow_calculation();
//     42
// });
// assert_eq!(x, 42);
//
macro_rules! stopwatch {
    ( $name:expr, $( $x:tt )* ) => {
        match ::std::env::var("STOPWATCH") {
            Ok(ref v) if v == "1" => {
                use std::time::SystemTime;
                let a = SystemTime::now();
                let value = { $($x)* };
                let b = SystemTime::now();
                let distance = {
                    let duration = if a > b {
                        a.duration_since(b)
                    } else {
                        b.duration_since(a)
                    }.unwrap();

                    (1000_000_000 * duration.as_secs() + duration.subsec_nanos() as u64) / 100_000
                };
                println!("[{}] {} ms ({}#{})", $name, distance, file!(), line!());
                value
            },
            _ =>
                { $($x)* }

        }
    };

    ( $( $x:tt )* ) => {
        stopwatch!("", { $($x)* })
    };
}