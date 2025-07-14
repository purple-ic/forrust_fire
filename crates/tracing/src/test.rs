#[cfg(not(feature = "serde"))]
compile_error!("`forrust_fire_tracing` requires the `serde` feature for testing");

#[rustfmt::skip]
mod tracing1;

#[test]
#[cfg(feature = "serde")]
fn tracing1() {
    use serde_json::json;

    use crate::providers::{ProviderExt, log::LogEventProvider};

    let ash_trayce = LogEventProvider::new().run(tracing1::run);

    let value = serde_json::to_value(ash_trayce).unwrap();
    println!("{value:#}");

    assert_eq!(
        value,
        json!({
          "0": {
            "v": {
              "ctx": {
                "message": "woa"
              },
              "file": "crates/tracing/src/test/tracing1.rs",
              "is_span": false,
              "level": "INFO",
              "line": 14,
              "mod": "forrust_fire_tracing::test::tracing1",
              "name": "event crates/tracing/src/test/tracing1.rs:14",
              "target": "forrust_fire_tracing::test::tracing1"
            }
          },
          "1": {
            "v": {
              "ctx": {
                "two_plus_two": "4"
              },
              "file": "crates/tracing/src/test/tracing1.rs",
              "is_span": true,
              "level": "WARN",
              "line": 16,
              "mod": "forrust_fire_tracing::test::tracing1",
              "name": "hello!",
              "target": "forrust_fire_tracing::test::tracing1"
            },
            "0": {
              "v": {
                "ctx": {
                  "message": "yaaa"
                },
                "file": "crates/tracing/src/test/tracing1.rs",
                "is_span": false,
                "level": "TRACE",
                "line": 17,
                "mod": "forrust_fire_tracing::test::tracing1",
                "name": "event crates/tracing/src/test/tracing1.rs:17",
                "target": "forrust_fire_tracing::test::tracing1"
              }
            },
            "1": {
              "v": {
                "ctx": {},
                "file": "crates/tracing/src/test/tracing1.rs",
                "is_span": true,
                "level": "INFO",
                "line": 8,
                "mod": "forrust_fire_tracing::test::tracing1",
                "name": "funkabloid",
                "target": "forrust_fire_tracing::test::tracing1"
              },
              "0": {
                "v": {
                  "ctx": {
                    "message": "hello world!"
                  },
                  "file": "crates/tracing/src/test/tracing1.rs",
                  "is_span": false,
                  "level": "DEBUG",
                  "line": 10,
                  "mod": "forrust_fire_tracing::test::tracing1",
                  "name": "event crates/tracing/src/test/tracing1.rs:10",
                  "target": "forrust_fire_tracing::test::tracing1"
                }
              },
              "1": {
                "v": {
                  "ctx": {
                    "message": "hello world, but in an evil manner"
                  },
                  "file": "crates/tracing/src/test/tracing1.rs",
                  "is_span": false,
                  "level": "ERROR",
                  "line": 11,
                  "mod": "forrust_fire_tracing::test::tracing1",
                  "name": "event crates/tracing/src/test/tracing1.rs:11",
                  "target": "forrust_fire_tracing::test::tracing1"
                }
              }
            },
          }
        })
    );
}
