//! Tests for the macOS crash-isolation mechanism — verifying that
//! `objc2::exception::catch` actually catches NSExceptions and that the outer
//! `panic::catch_unwind` defangs Rust panics. AE6 fidelity: this exercises the
//! mechanism that handles the *helper-process* case (non-nil bundleId but UN
//! context invalid), not just the simpler bundle-nil bypass.

use std::panic::{AssertUnwindSafe, catch_unwind};

use objc2_foundation::{NSException, ns_string};

#[test]
fn exception_catch_returns_err_when_an_objc_exception_is_thrown() {
    let result = objc2::exception::catch(|| {
        let exception = NSException::new(ns_string!("notify-status test"), None, None)
            .expect("NSException should be constructible in the test process");
        objc2::exception::throw(NSException::into_exception(exception));
    });

    assert!(result.is_err(), "expected exception::catch to return Err");
}

#[test]
fn nested_catchers_defang_a_rust_panic_inside_exception_catch_closure() {
    // `objc2::exception::catch` does not catch Rust panics — they propagate
    // out. The outer `catch_unwind(AssertUnwindSafe(...))` must intercept.
    let result = catch_unwind(AssertUnwindSafe(|| {
        objc2::exception::catch(|| {
            panic!("deliberate Rust panic inside exception::catch closure");
        })
    }));

    assert!(
        result.is_err(),
        "outer catch_unwind must intercept the propagated panic"
    );
}

#[test]
fn nested_catchers_succeed_for_a_clean_closure() {
    let result = catch_unwind(AssertUnwindSafe(|| objc2::exception::catch(|| 42_i64)));

    match result {
        Ok(Ok(value)) => assert_eq!(value, 42),
        Ok(Err(_)) => panic!("unexpected NSException"),
        Err(_) => panic!("unexpected Rust panic"),
    }
}
