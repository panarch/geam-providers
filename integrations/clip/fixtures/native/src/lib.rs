//! Application-owned exit adapter shared by standalone and embedding fixtures.

#[geam::provider(package = "geam_clip_search", modules = [native])]
pub struct Component;

#[geam::module(path = "geam_clip_search")]
mod native {
    use geam::provider::{BigInt, Call, ExitStatus, HostResult};

    #[geam::function]
    fn exit(#[geam::call] call: &mut Call<()>, status: BigInt) -> HostResult<()> {
        call.exit(ExitStatus::try_from(&status)?)
    }
}
