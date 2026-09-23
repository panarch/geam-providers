use geam::host::native::{NativeCall, NativeRules};
use geam::provider::{BigInt, BitArrayValue};
use geam::{
    HostCall, HostCallCompletion, HostCallError, HostComponentProfile, HostFailure, HostProvider,
    HostProviderComponent, HostProviderComponentInitialization, HostProviderComponentRegistration,
    HostProviderConfiguration, HostProviderInitializationError, HostProviderModule,
    HostRegistrationError, HostTypeIndex0, HostTypeList, HostTypeListEnd, HostTypeParameter,
    HostValue,
};
use geam_core::provider_support::bit_array_byte_slice;

pub struct Component;

impl HostProviderComponent for Component {
    const ID: &'static str = "houdini";
    type Stores = ();
    type RunState = ();
}

impl HostProviderComponentInitialization for Component {
    fn initialize(
        _: &HostProviderConfiguration,
    ) -> Result<Self::RunState, HostProviderInitializationError> {
        Ok(())
    }
}

impl<Profile> HostProvider<Profile> for Component
where
    Profile: HostComponentProfile<Component>,
{
    type State = ();

    fn project(state: &mut Profile::RunState) -> &mut Self::State {
        Profile::component_state(state)
    }
}

type Output = HostTypeParameter<0>;
type Input = HostTypeParameter<1>;
type Targets = HostTypeList<Output, HostTypeListEnd>;

fn coerce<'call, Profile>(
    mut call: NativeCall<'call, Profile, Component, Output, Targets>,
    input: HostValue<'call, Input>,
) -> Result<HostCallCompletion<'call, Output>, HostCallError>
where
    Profile: HostComponentProfile<Component>,
{
    let source = call.source::<Input>(input);
    let converted = call.convert::<HostTypeIndex0>(&source).ok_or_else(|| {
        HostCallError::from(HostFailure::new("houdini coerce: invalid conversion"))
    })?;
    Ok(call.finish(converted))
}

fn slice<'call, Profile>(
    call: HostCall<'call, Profile, Component, BitArrayValue>,
    input: BitArrayValue,
    from: BigInt,
    size: BigInt,
) -> Result<HostCallCompletion<'call, BitArrayValue>, HostCallError>
where
    Profile: HostComponentProfile<Component>,
{
    let sliced = slice_bytes(&input, &from, &size)?;
    Ok(call.return_value(sliced))
}

fn slice_bytes(
    input: &BitArrayValue,
    from: &BigInt,
    size: &BigInt,
) -> Result<BitArrayValue, HostFailure> {
    let start = usize::try_from(from).map_err(|_| invalid_slice())?;
    let length = usize::try_from(size).map_err(|_| invalid_slice())?;
    bit_array_byte_slice(input, start, length).ok_or_else(invalid_slice)
}

fn invalid_slice() -> HostFailure {
    HostFailure::new("houdini binary:part/3: invalid byte range")
}

impl<Profile> HostProviderComponentRegistration<Profile> for Component
where
    Profile: HostComponentProfile<Component>,
{
    fn providers() -> Result<Vec<HostProviderModule<Profile>>, HostRegistrationError> {
        HostProviderModule::new("houdini", "houdini/internal/escape_erl")
            .and_then(|module| {
                module.with_native_function::<Component, (Input,), Output, Targets, _>(
                    "coerce",
                    NativeRules::default(),
                    coerce::<Profile>,
                )
            })
            .and_then(|module| {
                module.with_scoped_function::<
                    Component,
                    (BitArrayValue, BigInt, BigInt),
                    BitArrayValue,
                    _,
                >("slice", slice::<Profile>)
            })
            .map(|module| vec![module])
    }
}

#[cfg(test)]
mod tests {
    use super::{BigInt, BitArrayValue, slice_bytes};

    #[test]
    fn byte_slice_retains_the_input_storage() {
        let input = BitArrayValue::from_bytes(b"prefix<suffix".to_vec());
        let sliced = slice_bytes(&input, &6.into(), &7.into()).expect("valid byte range");
        assert_eq!(sliced.bytes(), b"<suffix");
        assert!(std::ptr::eq(
            sliced.bytes().as_ptr(),
            input.bytes()[6..].as_ptr()
        ));
        assert_eq!(
            slice_bytes(&input, &0.into(), &0.into())
                .expect("empty byte range")
                .bytes(),
            b""
        );
        assert_eq!(
            slice_bytes(&input, &0.into(), &13.into())
                .expect("full byte range")
                .bytes(),
            input.bytes()
        );
    }

    #[test]
    fn invalid_byte_ranges_return_host_failures() {
        let input = BitArrayValue::from_bytes(vec![1, 2, 3]);
        let beyond_usize = BigInt::from(usize::MAX) + BigInt::from(1u8);
        let cases = [
            (BigInt::from(-1), BigInt::from(1)),
            (BigInt::from(0), BigInt::from(-1)),
            (beyond_usize.clone(), BigInt::from(1)),
            (BigInt::from(0), beyond_usize),
            (BigInt::from(usize::MAX), BigInt::from(2)),
            (BigInt::from(2), BigInt::from(2)),
        ];
        for (from, size) in cases {
            let failure = slice_bytes(&input, &from, &size).expect_err("invalid range");
            assert_eq!(
                failure.to_string(),
                "houdini binary:part/3: invalid byte range"
            );
        }

        let unaligned = BitArrayValue::try_from_parts(vec![0b1010_0000], 4)
            .expect("four-bit input fits one byte");
        assert!(slice_bytes(&unaligned, &0.into(), &0.into()).is_err());
    }
}
