mod stream;

use crate::{Call, Component, GrampsProfile};
use geam::__macro_support::ProviderValue;
use geam::execution::{ExecutionUnit, ExecutionUnitId, HostExecutionState, UnitExit};
use geam::gleam_erlang::{
    Atom, AtomSchema, Component as ErlangComponent, Pid, service::CurrentProcess,
};
use geam::host::{
    HostCallCompletion, HostCallError, HostConstructions, HostCustom, HostExternal,
    HostProviderModule, HostRegistrationError, HostTypeIndex0, HostTypeList, HostTypeListEnd,
};
use geam::provider::{BigInt, HostFailure, HostResult, advanced::NativeValue};
use std::collections::BTreeMap;

/// The execution domain owns streams; external values retain only keys.
#[derive(Default)]
pub struct Domain {
    next: BigInt,
    resources: BTreeMap<Key, Resource>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key {
    creator: ExecutionUnitId,
    serial: BigInt,
}

struct Resource {
    owner: ExecutionUnit,
    stream: Option<stream::Stream>,
}

impl Domain {
    fn open(&mut self, creator: ExecutionUnit) -> Key {
        let key = Key {
            creator: creator.id(),
            serial: self.next.clone(),
        };
        self.next += 1;
        self.resources.insert(
            key.clone(),
            Resource {
                owner: creator,
                stream: None,
            },
        );
        key
    }

    fn owned(&mut self, key: &Key, caller: &ExecutionUnit) -> HostResult<&mut Resource> {
        let resource = self.resources.get_mut(key).ok_or_else(|| {
            HostFailure::new("compression context is closed or belongs to another execution")
        })?;
        if resource.owner.id() != caller.id() {
            return Err(HostFailure::new("compression context belongs to another process").into());
        }
        Ok(resource)
    }
}

impl Resource {
    fn initialize(&mut self, create: fn() -> stream::Stream) -> HostResult<()> {
        if self.stream.is_some() {
            return Err(HostFailure::new("compression context is already initialized").into());
        }
        self.stream = Some(create());
        Ok(())
    }

    fn stream(&mut self) -> HostResult<&mut stream::Stream> {
        self.stream
            .as_mut()
            .ok_or_else(|| HostFailure::new("compression context is not initialized").into())
    }
}

impl HostExecutionState for Domain {
    fn started(&mut self, _: ExecutionUnit) {}
    fn finished(&mut self, unit: ExecutionUnitId, _: &UnitExit) {
        self.resources
            .retain(|_, resource| resource.owner.id() != unit);
    }
    fn close(&mut self) {
        self.resources.clear();
    }
}

#[geam::module(path = "gramps/websocket/compression", crate_path = geam, profile = crate::GrampsProfile,
    component = crate::Component, stores = compression)]
pub mod bindings {
    use super::{Component, Key};
    use geam::gleam_erlang::service::ProcessCall;
    use geam::gleam_stdlib::service;
    use geam::provider::{BitArrayValue, Call, HostFailure, HostResult};

    #[geam::external(name = "CompressionContext")]
    #[derive(PartialEq, Eq, Hash)]
    pub struct CompressionContext {
        pub(super) key: Key,
    }

    #[allow(dead_code)]
    #[geam::custom(input = ContextInput)]
    pub enum Context {
        Context {
            context: CompressionContext,
            no_takeover: bool,
        },
    }
    #[allow(dead_code)]
    #[geam::custom(input = DefaultInput)]
    pub enum Default {
        Default,
    }
    #[allow(dead_code)]
    #[geam::custom(input = DeflatedInput)]
    pub enum Deflated {
        Deflated,
    }
    #[allow(dead_code)]
    #[geam::custom(input = FlushInput)]
    enum Flush {
        Sync,
    }

    #[geam::function(profile = Profile)]
    fn do_inflate(
        #[geam::call] call: &mut Call<()>,
        context: &CompressionContext,
        data: BitArrayValue,
    ) -> HostResult<service::BytesTreeOutput> {
        let bytes = aligned_bytes(&data)?;
        call.current_process().and_then(|owner| {
            let stream = call
                .host_call()
                .service::<Component>()
                .owned(&context.key, &owner.execution_unit())?
                .stream()?;
            stream.process(bytes, true).map(|output| {
                service::BytesTreeOutput::from_bit_array(BitArrayValue::from_bytes(output))
            })
        })
    }

    #[geam::function(profile = Profile)]
    fn do_deflate(
        #[geam::call] call: &mut Call<()>,
        context: &CompressionContext,
        data: BitArrayValue,
        _: FlushInput,
    ) -> HostResult<service::BytesTreeOutput> {
        let bytes = aligned_bytes(&data)?;
        call.current_process().and_then(|owner| {
            let stream = call
                .host_call()
                .service::<Component>()
                .owned(&context.key, &owner.execution_unit())?
                .stream()?;
            stream.process(bytes, false).map(|output| {
                service::BytesTreeOutput::from_bit_array(BitArrayValue::from_bytes(output))
            })
        })
    }

    #[geam::function(profile = Profile)]
    fn do_close(#[geam::call] call: &mut Call<()>, context: &CompressionContext) -> HostResult<()> {
        call.current_process().and_then(|owner| {
            let domain = call.host_call().service::<Component>();
            domain.owned(&context.key, &owner.execution_unit())?;
            domain.resources.remove(&context.key);
            Ok(())
        })
    }

    #[geam::function(profile = Profile)]
    fn inflate_reset(
        #[geam::call] call: &mut Call<()>,
        context: &CompressionContext,
    ) -> HostResult<()> {
        call.current_process().and_then(|owner| {
            call.host_call()
                .service::<Component>()
                .owned(&context.key, &owner.execution_unit())?
                .stream()?
                .reset(true)
        })
    }

    #[geam::function(profile = Profile)]
    fn deflate_reset(
        #[geam::call] call: &mut Call<()>,
        context: &CompressionContext,
    ) -> HostResult<()> {
        call.current_process().and_then(|owner| {
            call.host_call()
                .service::<Component>()
                .owned(&context.key, &owner.execution_unit())?
                .stream()?
                .reset(false)
        })
    }

    fn aligned_bytes(data: &BitArrayValue) -> HostResult<&[u8]> {
        if data.bit_len().is_multiple_of(8) {
            Ok(data.bytes())
        } else {
            Err(HostFailure::new("compression input must contain complete bytes").into())
        }
    }
}

type Handle = <bindings::CompressionContext as ProviderValue>::Host;
type HandleSchema =
    <bindings::CompressionContext as geam::__macro_support::ProviderExternalDeclaration>::Schema;
impl<Profile: GrampsProfile> geam::HostExternalBinding<Profile, HandleSchema> for Component {
    type Storage = bindings::__GeamExternalStorage0;
}
type Context = <bindings::Context as ProviderValue>::Host;
type DefaultSetting = <bindings::Default as ProviderValue>::Host;
type Deflated = <bindings::Deflated as ProviderValue>::Host;
type AtomTargets = HostTypeList<Atom, HostTypeListEnd>;

pub(super) fn provider<Profile: GrampsProfile>()
-> Result<HostProviderModule<Profile>, HostRegistrationError> {
    bindings::__geam_module::<Profile>()
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (), Handle, HostTypeList<Handle, HostTypeListEnd>, _>("open", open::<Profile>))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (Handle, BigInt), Atom, AtomTargets, _>("inflate_init", inflate_init::<Profile>))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (Handle, DefaultSetting, Deflated, BigInt, BigInt, DefaultSetting), Atom, AtomTargets, _>("deflate_init", deflate_init::<Profile>))
        .and_then(|module| module.with_scoped_function_and_constructions::<Component, (Context, Pid), Atom, AtomTargets, _>("set_controlling_process", handoff::<Profile>))
}

fn open<'call, Profile: GrampsProfile>(
    call: Call<'call, Profile, Handle>,
    tokens: HostConstructions<'call, HostTypeList<Handle, HostTypeListEnd>>,
) -> Result<HostCallCompletion<'call, Handle>, HostCallError> {
    CurrentProcess::with(call, |mut process| {
        let creator = process.current().clone();
        let key = process.call().service::<Component>().open(creator);
        let handle = process
            .call()
            .construct_external_with_binding::<Component, HandleSchema, HostTypeListEnd>(
                tokens.at::<HostTypeIndex0>(),
                bindings::CompressionContext { key },
            );
        Ok(process.into_call().return_value(handle))
    })
}

fn ok<'call, Profile: GrampsProfile>(
    mut call: Call<'call, Profile, Atom>,
    tokens: HostConstructions<'call, AtomTargets>,
) -> HostCallCompletion<'call, Atom> {
    let atom = call
        .construct_external_with_binding::<ErlangComponent<Profile>, AtomSchema, HostTypeListEnd>(
            tokens.at::<HostTypeIndex0>(),
            NativeValue::symbol("ok"),
        );
    call.return_value(atom)
}

fn inflate_init<'call, Profile: GrampsProfile>(
    mut call: Call<'call, Profile, Atom>,
    tokens: HostConstructions<'call, AtomTargets>,
    handle: HostExternal<'call, Handle>,
    bits: BigInt,
) -> Result<HostCallCompletion<'call, Atom>, HostCallError> {
    if bits != BigInt::from(-15) {
        return Err(HostFailure::new("compression requires raw DEFLATE window bits -15").into());
    }
    let key = call.external_payload(handle).key.clone();
    call.require_execution_unit()
        .and_then(|owner| {
            call.service::<Component>()
                .owned(&key, &owner)?
                .initialize(stream::Stream::inflater)
        })
        .map(|()| ok(call, tokens))
}

#[allow(
    clippy::too_many_arguments,
    reason = "the original native declaration has six arguments, plus call and construction capabilities"
)]
fn deflate_init<'call, Profile: GrampsProfile>(
    mut call: Call<'call, Profile, Atom>,
    tokens: HostConstructions<'call, AtomTargets>,
    handle: HostExternal<'call, Handle>,
    _: HostCustom<'call, DefaultSetting>,
    _: HostCustom<'call, Deflated>,
    bits: BigInt,
    mem_level: BigInt,
    _: HostCustom<'call, DefaultSetting>,
) -> Result<HostCallCompletion<'call, Atom>, HostCallError> {
    if bits != BigInt::from(-15) {
        return Err(HostFailure::new("compression requires raw DEFLATE window bits -15").into());
    }
    if mem_level != BigInt::from(8) {
        return Err(HostFailure::new("compression requires memory level 8").into());
    }
    let key = call.external_payload(handle).key.clone();
    call.require_execution_unit()
        .and_then(|owner| {
            call.service::<Component>()
                .owned(&key, &owner)?
                .initialize(stream::Stream::deflater)
        })
        .map(|()| ok(call, tokens))
}

fn handoff<'call, Profile: GrampsProfile>(
    call: Call<'call, Profile, Atom>,
    tokens: HostConstructions<'call, AtomTargets>,
    context: HostCustom<'call, Context>,
    pid: HostExternal<'call, Pid>,
) -> Result<HostCallCompletion<'call, Atom>, HostCallError> {
    CurrentProcess::with(call, |mut process| {
        let owner = process.current().clone();
        let (handle, (_, ())) = process
            .call()
            .provider_borrow_remaining_custom_fields::<bindings::__GeamCustom0Constructor0>(
                context,
            );
        let key = process.call().external_payload(handle).key.clone();
        let target = process.processes().pid(pid);
        if !process.processes().is_alive(&target) {
            return Err(HostFailure::new(
                "compression owner must be a live process in this execution",
            )
            .into());
        }
        process
            .call()
            .service::<Component>()
            .owned(&key, &owner)?
            .owner = target;
        Ok(ok(process.into_call(), tokens))
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::test_support::{Profile, execution_host::TestHost, source_project};
    use crate::{Call, Component};
    use geam::execution::{ExecutionUnit, ExecutionUnitId, HostExecutionState, UnitExit};
    use geam::provider::BigInt;
    use geam::{HostCallCompletion, HostCallError, HostProviderModule};
    use std::sync::{Arc, Mutex};

    pub(crate) type LifecycleAudit = Arc<Mutex<Audit>>;

    #[derive(Default)]
    pub(crate) struct Audit {
        pub(crate) events: Vec<(String, usize, usize)>,
        trees: Vec<geam::gleam_stdlib::service::BytesTreeInput>,
        context: Option<
            geam::__macro_support::ProviderOwnedExternal<super::bindings::CompressionContext>,
        >,
        pid: Option<ExecutionUnit>,
    }

    /// Observe the production service's hooks through the public execution driver.
    /// The audit owns no streams and adds no production or scheduler test branches.
    #[derive(Default)]
    pub(crate) struct AuditedDomain {
        pub(crate) domain: super::Domain,
        pub(crate) audit: LifecycleAudit,
    }

    impl HostExecutionState for AuditedDomain {
        fn started(&mut self, unit: ExecutionUnit) {
            self.domain.started(unit);
        }

        fn finished(&mut self, unit: ExecutionUnitId, exit: &UnitExit) {
            let before = self.domain.resources.len();
            self.domain.finished(unit, exit);
            self.audit.lock().unwrap().events.push((
                format!("{exit:?}"),
                before,
                self.domain.resources.len(),
            ));
        }

        fn close(&mut self) {
            let before = self.domain.resources.len();
            self.domain.close();
            self.audit.lock().unwrap().events.push((
                "close".into(),
                before,
                self.domain.resources.len(),
            ));
        }
    }

    const PRIVATE_CALLERS: &str = r#"
pub fn probe_open() { Context(open(),False) }
pub fn probe_computed_open() { Context(invoke_open(open),False) }
fn invoke_open(create: fn() -> CompressionContext) { create() }
pub fn probe_inflate_init(context: Context, bits: Int) { inflate_init(context.context,bits) }
pub fn probe_deflate_init(context: Context, bits: Int, memory: Int) { deflate_init(context.context,Default,Deflated,bits,memory,Default) }
pub fn probe_inflate(context: Context, data: BitArray) { do_inflate(context.context,data) }
pub fn probe_deflate(context: Context, data: BitArray) { do_deflate(context.context,data,Sync) }
pub fn probe_inflate_reset(context: Context) { inflate_reset(context.context) }
pub fn probe_deflate_reset(context: Context) { deflate_reset(context.context) }
"#;

    fn count<'call>(
        mut call: Call<'call, Profile, BigInt>,
    ) -> Result<HostCallCompletion<'call, BigInt>, HostCallError> {
        let count = call.service::<Component>().resources.len();
        Ok(call.return_value(count.into()))
    }
    fn close_domain<'call>(
        mut call: Call<'call, Profile, ()>,
    ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
        use geam::execution::HostExecutionState;
        call.service::<Component>().close();
        assert_eq!(call.service::<Component>().resources.len(), 0);
        Ok(call.return_value(()))
    }

    fn cancel<'call>(
        call: Call<'call, Profile, ()>,
    ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
        call.require_execution_unit().map(|unit| {
            assert!(unit.cancel());
            call.return_value(())
        })
    }

    fn exit<'call>(
        call: Call<'call, Profile, ()>,
    ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
        call.exit(geam::execution::ExitStatus::new(7))
    }

    fn attempt<'call>(
        call: Call<'call, Profile, geam::StringValue>,
        tokens: geam::HostConstructions<'call, geam::HostTypeListEnd>,
        body: geam::HostCallable<'call, geam::HostTypeListEnd, ()>,
    ) -> Result<geam::HostCallContinuation<'call, geam::StringValue>, HostCallError> {
        let body = call.owned_callable(body, &tokens);
        Ok(call.resume(tokens, move |context| {
            Box::pin(async move {
                let result = body.invoke(&context, |_, _| (), |_, _, ()| Ok(())).await;
                let reason: geam::StringValue = result
                    .err()
                    .map(|error| error.to_string())
                    .unwrap_or_default()
                    .into();
                Ok(geam::HostOwnedCompletion::new(move |call, _| {
                    Ok(call.return_value(reason))
                }))
            })
        }))
    }

    fn retain_tree<'call>(
        mut call: Call<'call, Profile, ()>,
        tree: geam::HostCustom<'call, <geam::gleam_stdlib::service::BytesTreeInput as geam::__macro_support::ProviderValue>::Host>,
    ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
        use geam::__macro_support::{ProviderConstructions, ProviderInputValue};
        let tree = geam::gleam_stdlib::service::BytesTreeInput::from_host_with(
            &mut call,
            tree,
            &ProviderConstructions::none(),
        );
        call.execution_state()
            .rest
            .audit
            .lock()
            .unwrap()
            .trees
            .push(tree);
        Ok(call.return_value(()))
    }

    fn retain_context<'call>(
        mut call: Call<'call, Profile, ()>,
        handle: geam::HostExternal<'call, super::Handle>,
    ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
        let retained = call
            .provider_external_item_with::<Component, super::HandleSchema, geam::HostTypeListEnd>(
                handle,
            );
        call.execution_state().rest.audit.lock().unwrap().context = Some(retained);
        Ok(call.return_value(()))
    }

    fn restore_context<'call>(
        mut call: Call<'call, Profile, super::Handle>,
        tokens: geam::HostConstructions<
            'call,
            geam::HostTypeList<super::Handle, geam::HostTypeListEnd>,
        >,
    ) -> Result<HostCallCompletion<'call, super::Handle>, HostCallError> {
        let retained = call
            .execution_state()
            .rest
            .audit
            .lock()
            .unwrap()
            .context
            .as_ref()
            .unwrap()
            .clone();
        // Leases belong to their original store. Construct this logical identity
        // through its real producer in the second store; never recreate a stream
        // or invent a process identity for this independent-domain test.
        let payload = retained.with(|context| super::bindings::CompressionContext {
            key: context.key.clone(),
        });
        let handle = call.construct_external_with_binding::<Component, super::HandleSchema, geam::HostTypeListEnd>(tokens.at::<geam::HostTypeIndex0>(), payload);
        Ok(call.return_value(handle))
    }

    fn retain_pid<'call>(
        mut call: Call<'call, Profile, ()>,
        pid: geam::HostExternal<'call, geam::gleam_erlang::Pid>,
    ) -> Result<HostCallCompletion<'call, ()>, HostCallError> {
        let unit = geam::gleam_erlang::service::Processes::new(&mut call).pid(pid);
        call.execution_state().rest.audit.lock().unwrap().pid = Some(unit);
        Ok(call.return_value(()))
    }

    fn restore_pid<'call>(
        mut call: Call<'call, Profile, geam::gleam_erlang::Pid>,
        tokens: geam::HostConstructions<
            'call,
            geam::HostTypeList<geam::gleam_erlang::Pid, geam::HostTypeListEnd>,
        >,
    ) -> Result<HostCallCompletion<'call, geam::gleam_erlang::Pid>, HostCallError> {
        let unit = call
            .execution_state()
            .rest
            .audit
            .lock()
            .unwrap()
            .pid
            .as_ref()
            .unwrap()
            .clone();
        let pid = geam::gleam_erlang::service::pid_value(
            &mut call,
            tokens.at::<geam::HostTypeIndex0>(),
            unit,
        );
        Ok(call.return_value(pid))
    }

    fn probes() -> HostProviderModule<Profile> {
        HostProviderModule::new("fixture", "fixture")
            .unwrap()
            .with_scoped_function::<Component, (), BigInt, _>("count", count)
            .unwrap()
            .with_scoped_function::<Component, (), (), _>("close_domain", close_domain)
            .unwrap()
    }

    #[test]
    fn initialization_alignment_mode_closed_alias_and_reset_failures_remain_host_failures() {
        for (operation, reason) in [
            (
                "c.probe_inflate_init(ctx,15)",
                "raw DEFLATE window bits -15",
            ),
            (
                "c.probe_deflate_init(ctx,15,8)",
                "raw DEFLATE window bits -15",
            ),
            ("c.probe_deflate_init(ctx,-15,9)", "memory level 8"),
            ("c.probe_inflate(ctx,<<>>)", "not initialized"),
            ("c.probe_deflate(ctx,<<>>)", "not initialized"),
            ("c.probe_inflate_reset(ctx)", "not initialized"),
            ("c.probe_deflate_reset(ctx)", "not initialized"),
            (
                "c.probe_inflate_init(ctx,-15) c.probe_inflate_init(ctx,-15)",
                "already initialized",
            ),
            (
                "c.probe_deflate_init(ctx,-15,8) c.probe_deflate_init(ctx,-15,8)",
                "already initialized",
            ),
            ("c.probe_inflate(ctx,<<1:1>>)", "complete bytes"),
            ("c.probe_deflate(ctx,<<1:1>>)", "complete bytes"),
            (
                "c.probe_inflate_init(ctx,-15) c.probe_deflate(ctx,<<>>)",
                "does not match",
            ),
            (
                "c.probe_deflate_init(ctx,-15,8) c.probe_inflate(ctx,<<>>)",
                "does not match",
            ),
            (
                "c.probe_inflate_init(ctx,-15) c.probe_deflate_reset(ctx)",
                "does not match",
            ),
            (
                "c.probe_deflate_init(ctx,-15,8) c.probe_inflate_reset(ctx)",
                "does not match",
            ),
            ("c.close(ctx) c.close(ctx)", "closed or belongs"),
            (
                "c.close(ctx) c.probe_deflate(ctx,<<>>)",
                "closed or belongs",
            ),
            (
                "c.close(ctx) c.probe_inflate_init(ctx,-15)",
                "closed or belongs",
            ),
            (
                "c.close(ctx) c.probe_deflate_init(ctx,-15,8)",
                "closed or belongs",
            ),
            (
                "c.close(ctx) c.probe_inflate_reset(ctx)",
                "closed or belongs",
            ),
            (
                "c.close(ctx) c.probe_deflate_reset(ctx)",
                "closed or belongs",
            ),
        ] {
            let source = format!(
                "import gramps/websocket/compression as c\npub fn main() {{ let ctx=c.probe_open() {operation} Nil }}"
            );
            let (mut execution, mut state) = source_project(
                &source,
                &[("gramps/websocket/compression", PRIVATE_CALLERS)],
                [],
            );
            let host = TestHost::default();
            let error = host
                .block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .unwrap_err();
            assert!(error.to_string().contains(reason), "{operation}: {error}");
        }
    }

    #[test]
    fn mode_rejection_preserves_both_contexts_without_a_reset() {
        let source = r#"
import gleam/string
import gramps/websocket/compression as c
@external(erlang,"fixture","attempt") fn attempt(body: fn() -> Nil) -> String
pub fn main() {
  let ctx=c.init(c.ContextTakeover(False,False))
  let wire=c.deflate(ctx.deflate,<<"mode recovery":utf8>>)
  let assert True=string.contains(attempt(fn(){c.deflate(ctx.inflate,<<>>) Nil}),"does not match the initialized stream")
  let assert True=string.contains(attempt(fn(){c.inflate(ctx.deflate,wire) Nil}),"does not match the initialized stream")
  let assert <<"mode recovery":utf8>>=c.inflate(ctx.inflate,wire)
  let assert <<"dictionary continues":utf8>>=c.inflate(ctx.inflate,c.deflate(ctx.deflate,<<"dictionary continues":utf8>>))
  c.close(ctx.inflate)
  c.close(ctx.deflate)
  Nil
}
"#;
        let (mut execution, mut state) = source_project(
            source,
            &[],
            [HostProviderModule::new("fixture", "fixture")
                .unwrap()
                .with_resumable_function::<
                    Component,
                    (geam::HostFunctionType<geam::HostTypeListEnd, ()>,),
                    geam::StringValue,
                    geam::HostTypeListEnd,
                    _,
                >("attempt", attempt)
                .unwrap()],
        );
        assert_eq!(
            crate::test_support::execution_host::run(&mut execution, &mut state, &mut Vec::new())
                .unwrap(),
            geam::Value::Nil
        );
    }

    #[test]
    fn bytes_tree_survives_reset_close_and_domain_shutdown_and_handles_keep_source_identity() {
        let source = r#"
import gleam/bytes_tree
import gleam/dict
import gleam/string
import gleam/erlang/atom
import gleam/erlang/process
import gramps/websocket/compression as c
@external(erlang,"fixture","count") fn count() -> Int
@external(erlang,"fixture","close_domain") fn close_domain() -> Nil
pub fn main() {
  let ctx=c.probe_computed_open()
  let other=c.probe_open()
  let assert False=ctx.context==other.context
  let before=string.inspect(ctx.context)
  let values=dict.from_list([#(ctx.context,42)])
  let assert 2=count()
  let assert "ok"=atom.to_string(c.probe_deflate_init(ctx,-15,8))
  let assert "ok"=atom.to_string(c.set_controlling_process(ctx,process.self()))
  let tree=c.probe_deflate(ctx,<<"hello":utf8>>)
  let before_reset=bytes_tree.to_bit_array(tree)
  c.probe_deflate_reset(ctx)
  c.close(ctx)
  let assert 1=count()
  let assert Ok(42)=dict.get(values,ctx.context)
  let assert True=before==string.inspect(ctx.context)
  close_domain()
  let assert 0=count()
  let assert True=before_reset==bytes_tree.to_bit_array(tree)
  Nil
}
"#;
        let (mut execution, mut state) = source_project(
            source,
            &[("gramps/websocket/compression", PRIVATE_CALLERS)],
            [probes()],
        );
        let host = TestHost::default();
        assert_eq!(
            host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .unwrap()
                .try_into_value()
                .unwrap(),
            geam::Value::Nil
        );
    }

    #[test]
    fn current_owner_exit_and_kill_release_resources_after_handoff() {
        let source = r#"
import gleam/erlang/process
import gleam/erlang/atom
import gramps/websocket/compression as c
@external(erlang,"fixture","count") fn count() -> Int
@external(erlang,"fixture","close_domain") fn close_domain() -> Nil
pub fn main() {
  let reply=process.new_subject()
  let child=process.spawn_unlinked(fn() {
    let inbox=process.new_subject()
    process.send(reply,inbox)
    let ctx=process.receive_forever(inbox)
    let assert <<"hello":utf8>>=c.inflate(ctx,<<202,72,205,201,201,7,0>>)
    process.send(reply,inbox)
  })
  let inbox=process.receive_forever(reply)
  let ctx=c.init(c.ContextTakeover(False,False))
  let assert 2=count()
  let assert "ok"=atom.to_string(c.set_controlling_process(ctx.inflate,child))
  process.send(inbox,ctx.inflate)
  let _=process.receive_forever(reply)
  let monitor=process.monitor(child)
  let _=process.new_selector() |> process.select_specific_monitor(monitor,fn(_){Nil}) |> process.selector_receive_forever
  let assert 1=count()
  c.close(ctx.deflate)
  let ready=process.new_subject()
  let victim=process.spawn_unlinked(fn() {
    let _=c.init(c.ContextTakeover(False,False))
    process.send(ready,Nil)
    process.sleep_forever()
  })
  process.receive_forever(ready)
  let assert 2=count()
  let monitor=process.monitor(victim)
  process.kill(victim)
  let _=process.new_selector() |> process.select_specific_monitor(monitor,fn(_){Nil}) |> process.selector_receive_forever
  let assert 0=count()
  Nil
}
"#;
        let (mut execution, mut state) = source_project(source, &[], [probes()]);
        let host = TestHost::default();
        assert_eq!(
            host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()))
                .unwrap()
                .try_into_value()
                .unwrap(),
            geam::Value::Nil
        );
    }

    #[test]
    fn rejected_owners_and_failed_streams_preserve_the_live_owner_and_reset_recovers() {
        let source = r#"
import gleam/bytes_tree
import gleam/erlang/process
import gleam/string
import gramps/websocket/compression as c
@external(erlang,"fixture","attempt") fn attempt(body: fn() -> Nil) -> String
pub fn main() {
  let assert ""=attempt(fn(){Nil})
  let ready=process.new_subject()
  let done=process.new_subject()
  let child=process.spawn_unlinked(fn() {
    let inbox=process.new_subject()
    process.send(ready,inbox)
    let #(ctx,wire)=process.receive_forever(inbox)
    let assert <<"hello":utf8>>=c.inflate(ctx,wire)
    c.close(ctx)
    process.send(done,Nil)
  })
  let inbox=process.receive_forever(ready)
  let ctx=c.init(c.ContextTakeover(False,False))
  c.set_controlling_process(ctx.inflate,child)
  let assert True=string.contains(attempt(fn(){c.inflate(ctx.inflate,<<>>) Nil}),"belongs to another process")
  let assert True=string.contains(attempt(fn(){c.set_controlling_process(ctx.inflate,process.self()) Nil}),"belongs to another process")
  let wire=c.deflate(ctx.deflate,<<"hello":utf8>>)
  process.send(inbox,#(ctx.inflate,wire))
  process.receive_forever(done)
  let monitor=process.monitor(child)
  let _=process.new_selector() |> process.select_specific_monitor(monitor,fn(_){Nil}) |> process.selector_receive_forever
  // The completed child is a real Pid, not a synthetic missing identity.
  let another=c.init(c.ContextTakeover(False,False))
  let assert True=string.contains(attempt(fn(){c.set_controlling_process(another.inflate,child) Nil}),"live process in this execution")
  let assert <<"still owned":utf8>>=c.inflate(another.inflate,c.deflate(another.deflate,<<"still owned":utf8>>))
  c.close(another.inflate)
  c.close(another.deflate)
  c.close(ctx.deflate)
  let recovery=c.probe_open()
  c.probe_inflate_init(recovery,-15)
  let assert True=string.contains(attempt(fn(){c.probe_inflate(recovery,<<7>>) Nil}),"backend failed")
  let assert True=string.contains(attempt(fn(){c.probe_inflate(recovery,<<>>) Nil}),"needs reset")
  c.probe_inflate_reset(recovery)
  let tree=c.probe_inflate(recovery,<<243,72,205,201,201,7,0>>)
  let assert <<"Hello":utf8>>=bytes_tree.to_bit_array(tree)
  c.close(recovery)
  Nil
}
"#;
        let (mut execution, mut state) = source_project(
            source,
            &[("gramps/websocket/compression", PRIVATE_CALLERS)],
            [HostProviderModule::new("fixture", "fixture")
                .unwrap()
                .with_resumable_function::<
                    Component,
                    (geam::HostFunctionType<geam::HostTypeListEnd, ()>,),
                    geam::StringValue,
                    geam::HostTypeListEnd,
                    _,
                >("attempt", attempt)
                .unwrap()],
        );
        assert_eq!(
            crate::test_support::execution_host::run(&mut execution, &mut state, &mut Vec::new())
                .unwrap(),
            geam::Value::Nil
        );
    }

    #[test]
    fn the_old_owner_can_exit_without_closing_a_stream_owned_by_a_live_successor() {
        let source = r#"
import gleam/erlang/process
import gramps/websocket/compression as c
@external(erlang,"fixture","count") fn count() -> Int
@external(erlang,"fixture","close_domain") fn close_domain() -> Nil
pub fn main() {
  let reply=process.new_subject()
  let parent=process.self()
  let creator=process.spawn_unlinked(fn() {
    let ctx=c.init(c.ContextTakeover(False,False))
    c.set_controlling_process(ctx.deflate,parent)
    process.send(reply,ctx.deflate)
  })
  let ctx=process.receive_forever(reply)
  let monitor=process.monitor(creator)
  let _=process.new_selector() |> process.select_specific_monitor(monitor,fn(_){Nil}) |> process.selector_receive_forever
  let assert 1=count()
  let _=c.deflate(ctx,<<"owner survived":utf8>>)
  c.close(ctx)
  let assert 0=count()
  Nil
}
"#;
        let (mut execution, mut state) = source_project(source, &[], [probes()]);
        assert_eq!(
            crate::test_support::execution_host::run(&mut execution, &mut state, &mut Vec::new())
                .unwrap(),
            geam::Value::Nil
        );
    }

    #[test]
    fn independent_live_domains_reject_retained_handles_and_pids_and_trees_outlive_execution() {
        use geam::__macro_support::ProviderValue;
        use geam::provider::StringValue;
        use geam::{HostFunctionType, HostTypeListEnd};
        type Tree = <geam::gleam_stdlib::service::BytesTreeInput as ProviderValue>::Host;
        let retaining = HostProviderModule::new("fixture", "fixture")
            .unwrap()
            .with_scoped_function::<Component, (super::Handle,), (), _>(
                "retain_context",
                retain_context,
            )
            .unwrap()
            .with_scoped_function::<Component, (geam::gleam_erlang::Pid,), (), _>(
                "retain_pid",
                retain_pid,
            )
            .unwrap()
            .with_scoped_function::<Component, (Tree,), (), _>("retain_tree", retain_tree)
            .unwrap();
        let first = r#"
import gleam/bytes_tree
import gleam/erlang/process
import gramps/websocket/compression as c
@external(erlang,"fixture","retain_context") fn retain_context(ctx: c.CompressionContext) -> Nil
@external(erlang,"fixture","retain_pid") fn retain_pid(pid: process.Pid) -> Nil
@external(erlang,"fixture","retain_tree") fn retain_tree(tree: bytes_tree.BytesTree) -> Nil
pub fn main() {
  let ctx=c.init(c.ContextTakeover(False,False))
  retain_context(ctx.inflate.context)
  retain_pid(process.self())
  retain_tree(c.probe_inflate(ctx.inflate,<<243,72,205,201,201,7,0>>))
  process.sleep_forever()
}
"#;
        let (mut execution, mut state) = source_project(
            first,
            &[("gramps/websocket/compression", PRIVATE_CALLERS)],
            [retaining],
        );
        let audit = Arc::clone(&state.audit);
        let host = TestHost::default();
        let mut echo = Vec::new();
        {
            let mut running = std::pin::pin!(execution.run_main(&host, &mut state, &mut echo));
            assert!(host.poll(running.as_mut()).is_pending());
            let unit = audit.lock().unwrap().pid.as_ref().unwrap().clone();
            assert!(unit.is_active());
            assert_eq!(
                audit.lock().unwrap().trees[0].to_bit_array().bytes(),
                b"Hello"
            );

            let restoring = HostProviderModule::new("fixture", "fixture").unwrap()
                .with_scoped_function_and_constructions::<Component, (), super::Handle, geam::HostTypeList<super::Handle, HostTypeListEnd>, _>("restore_context", restore_context).unwrap()
                .with_scoped_function_and_constructions::<Component, (), geam::gleam_erlang::Pid, geam::HostTypeList<geam::gleam_erlang::Pid, HostTypeListEnd>, _>("restore_pid", restore_pid).unwrap()
                .with_resumable_function::<Component, (HostFunctionType<HostTypeListEnd, ()>,), StringValue, HostTypeListEnd, _>("attempt", attempt).unwrap();
            let second = r#"
import gleam/erlang/process
import gleam/string
import gramps/websocket/compression as c
@external(erlang,"fixture","restore_context") fn restore_context() -> c.CompressionContext
@external(erlang,"fixture","restore_pid") fn restore_pid() -> process.Pid
@external(erlang,"fixture","attempt") fn attempt(body: fn() -> Nil) -> String
pub fn main() {
  let current=c.init(c.ContextTakeover(False,False))
  let foreign=c.Context(restore_context(),False)
  let assert True=string.contains(attempt(fn(){c.inflate(foreign,<<>>) Nil}),"closed or belongs to another execution")
  let foreign_pid=restore_pid()
  let assert False=process.is_alive(foreign_pid)
  let assert True=string.contains(attempt(fn(){c.set_controlling_process(current.inflate,foreign_pid) Nil}),"live process in this execution")
  let assert <<"current domain":utf8>>=c.inflate(current.inflate,c.deflate(current.deflate,<<"current domain":utf8>>))
  Nil
}
"#;
            let (mut independent, mut independent_state) = source_project(second, &[], [restoring]);
            independent_state.audit = Arc::clone(&audit);
            assert_eq!(
                host.block_on(independent.run_main(&host, &mut independent_state, &mut Vec::new()))
                    .unwrap()
                    .try_into_value()
                    .unwrap(),
                geam::Value::Nil
            );
            assert!(
                unit.is_active(),
                "a foreign domain must not terminate the first domain"
            );
            assert_eq!(
                audit.lock().unwrap().trees[0].to_bit_array().bytes(),
                b"Hello"
            );
            assert!(unit.cancel());
            assert_eq!(
                host.poll(running.as_mut())
                    .map(|result| result.unwrap_err().to_string()),
                std::task::Poll::Ready("the Gleam entry was cancelled".into())
            );
        }
        drop(execution);
        drop(state);
        let audit = audit.lock().unwrap();
        assert_eq!(audit.trees[0].to_bit_array().bytes(), b"Hello");
        assert_eq!(
            audit.events,
            vec![
                ("Completed".into(), 2, 0),
                ("close".into(), 0, 0),
                ("Cancelled".into(), 2, 0),
                ("close".into(), 0, 0)
            ]
        );
    }

    #[test]
    fn completion_failure_cancellation_and_application_exit_release_all_streams() {
        for (ending, outcome, finish) in [
            ("Nil", "Returned(Nil)", "Completed"),
            (
                "panic as \"stream owner failed\"",
                "panic: stream owner failed",
                "Failed",
            ),
            ("cancel()", "the Gleam entry was cancelled", "Cancelled"),
            ("exit()", "Exited(ExitStatus(7))", "Cancelled"),
        ] {
            let source = format!(
                r#"
import gramps/websocket/compression as c
@external(erlang,"fixture","count") fn count() -> Int
@external(erlang,"fixture","cancel") fn cancel() -> Nil
@external(erlang,"fixture","exit") fn exit() -> Nil
pub fn main() {{
  let _=c.init(c.ContextTakeover(False,False))
  let assert 2=count()
  {ending}
}}
"#
            );
            let provider = HostProviderModule::new("fixture", "fixture")
                .unwrap()
                .with_scoped_function::<Component, (), BigInt, _>("count", count)
                .unwrap()
                .with_scoped_function::<Component, (), (), _>("cancel", cancel)
                .unwrap()
                .with_scoped_function::<Component, (), (), _>("exit", exit)
                .unwrap();
            let (mut execution, mut state) = source_project(&source, &[], [provider]);
            let host = TestHost::default();
            let result = host.block_on(execution.run_main(&host, &mut state, &mut Vec::new()));
            let observed = result
                .map(|value| format!("{value:?}"))
                .unwrap_or_else(|error| error.to_string());
            assert!(observed.contains(outcome), "{ending}: {observed}");
            let audit = &state.audit.lock().unwrap().events;
            assert_eq!(audit.len(), 2, "{ending}: {audit:?}");
            assert!(audit[0].0.starts_with(finish), "{ending}: {audit:?}");
            assert_eq!((audit[0].1, audit[0].2), (2, 0), "{ending}");
            assert_eq!(audit[1], ("close".into(), 0, 0), "{ending}");
        }
    }
}
