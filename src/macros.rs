/// logs a single message at the level provided
#[macro_export]
macro_rules! log {
    ($level:expr, $message:expr) => {
        $crate::Log::new($level, $message).submit()
    };
    ($level:expr, $message:expr, $($key:expr => $value:expr),+) => {{
        let mut log = $crate::Log::new($level, $message);
        $(
            log.add($key, $value).unwrap();
        )+
        log.submit()
    }};
}

#[rustfmt::skip] // This macro confuses rustfmt and it tries to indent it with the inner lines very far to the right.
macro_rules! make_level_log_macro {
    // Passing the $ as a token is the workaround to allow parsing the repeated expression https://github.com/rust-lang/rust/issues/35853
    ($d:tt $name:ident, $level:ident, $docs:literal) => {
        
        #[doc = $docs]
        #[macro_export]
        macro_rules! $name {
            ($message:expr) => {
                $crate::log!($crate::Level::$level, $message)
            };
            ($message:expr, $d($d params:tt)*) => {{
                $crate::log!($crate::Level::$level, $message, $d($d params)*)
            }};
        }
    };
}

make_level_log_macro!($ error, Error, "logs a message with `Level::Error`");
make_level_log_macro!($ warn, Warning, "logs a message with `Level::Warning`");
make_level_log_macro!($ info, Info, "logs a message with `Level::Info`");
make_level_log_macro!($ debug, Debug, "logs a message with `Level::Debug`");
make_level_log_macro!($ trace, Trace, "logs a message with `Level::Trace`");

#[tokio::test]
async fn macro_tests() {
    use crate::{backend::Memory, Level, Manager};
    use std::time::Duration;

    let test_backend = Memory::new(2);
    let entries = test_backend.entries.clone();
    let destination = Manager::default()
        .with_backend(test_backend)
        .launch(|task| {
            tokio::spawn(task);
        });

    let tests = async move {
        log!(Level::Info, "A");
        tokio::time::sleep(Duration::from_millis(1)).await;
        {
            let entries = entries.lock().await;
            assert_eq!(entries[0].level, Level::Info);
            assert_eq!(entries[0].message, "A");
        }
        log!(Level::Info, "B", "a" => 1_u64);
        tokio::time::sleep(Duration::from_millis(1)).await;
        {
            let entries = entries.lock().await;
            assert_eq!(entries[0].level, Level::Info);
            assert_eq!(entries[0].payload, serde_json::json!({"a": 1_u64}));
        }

        macro_rules! test_log_level {
            ($macroname:ident, $level:expr) => {{
                $macroname!("A");
                tokio::time::sleep(Duration::from_millis(1)).await;
                {
                    let entries = entries.lock().await;
                    assert_eq!(entries[0].level, $level);
                    assert_eq!(entries[0].message, "A");
                }
                $macroname!("B", "a" => 1_u64);
                tokio::time::sleep(Duration::from_millis(1)).await;
                {
                    let entries = entries.lock().await;
                    assert_eq!(entries[0].level, $level);
                    assert_eq!(entries[0].payload, serde_json::json!({"a": 1_u64}));
                }
            }}
        }

        test_log_level!(error, Level::Error);
        test_log_level!(warn, Level::Warning);
        test_log_level!(info, Level::Info);
        test_log_level!(debug, Level::Debug);
        test_log_level!(trace, Level::Trace);
    };

    crate::Configuration::named("macros", destination)
        .run(tests)
        .await;
}
