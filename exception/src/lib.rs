//! Native functions for the unmodified `exception` 2.1.1 Gleam package.

use geam::provider::advanced::NativeValue;
use geam::provider::{Call, Callback, HostResult, Value};

#[geam::provider(package = "exception", modules = [exception])]
pub struct Component;

#[geam::module(path = "exception")]
mod exception {
    use super::{Call, Callback, HostResult, NativeValue, Value};

    // All three constructors belong to the upstream Gleam type. Geam has no
    // throw/exit failure classes to produce the latter two from a callback.
    #[allow(dead_code)]
    #[geam::custom]
    enum Exception {
        Errored(geam::gleam_stdlib::Dynamic),
        Thrown(geam::gleam_stdlib::Dynamic),
        Exited(geam::gleam_stdlib::Dynamic),
    }

    #[geam::function(await)]
    async fn rescue<Item>(
        #[geam::call] call: &mut Call<()>,
        body: Callback<fn() -> Value<Item>>,
    ) -> HostResult<Result<Value<Item>, Exception>> {
        match call.invoke(&body, ()).await {
            Ok(value) => Ok(Ok(value)),
            Err(error) => recover(error).map(Err),
        }
    }

    fn recover(error: geam::HostExecutionError) -> Result<Exception, geam::HostExecutionError> {
        match error {
            geam::HostExecutionError::Cancelled => Err(geam::HostExecutionError::Cancelled),
            failure @ (geam::HostExecutionError::Host(_)
            | geam::HostExecutionError::Execution(_)) => Ok(Exception::Errored(
                geam::gleam_stdlib::Dynamic::from_native(NativeValue::symbol(failure.to_string())),
            )),
        }
    }

    #[geam::function(await)]
    async fn defer<Item, Cleanup>(
        #[geam::call] call: &mut Call<()>,
        cleanup: Callback<fn() -> Value<Cleanup>>,
        body: Callback<fn() -> Value<Item>>,
    ) -> HostResult<Value<Item>> {
        let body_result = call.invoke(&body, ()).await;
        let _ = call.invoke(&cleanup, ()).await?;
        body_result
    }

    #[geam::function(await)]
    async fn on_crash<Item, Cleanup>(
        #[geam::call] call: &mut Call<()>,
        cleanup: Callback<fn() -> Value<Cleanup>>,
        body: Callback<fn() -> Value<Item>>,
    ) -> HostResult<Value<Item>> {
        let body_result = call.invoke(&body, ()).await;
        if matches!(
            body_result,
            Err(geam::HostExecutionError::Host(_) | geam::HostExecutionError::Execution(_))
        ) {
            let _ = call.invoke(&cleanup, ()).await?;
        }
        body_result
    }

    #[cfg(test)]
    mod tests {
        use super::{Exception, recover};
        use geam::{HostExecutionError, HostFailure};

        #[test]
        fn catchable_host_failure_has_an_inspectable_reason() {
            let result = recover(HostFailure::new("provider failed").into());
            assert!(matches!(
                result,
                Ok(Exception::Errored(reason))
                    if reason.native_value().as_symbol().as_deref() == Some("provider failed")
            ));
        }

        #[test]
        fn cancellation_does_not_become_a_gleam_exception() {
            let result = recover(HostExecutionError::Cancelled);
            assert_eq!(
                result.err().map(|error| std::mem::discriminant(&error)),
                Some(std::mem::discriminant(&HostExecutionError::Cancelled))
            );
        }
    }
}
