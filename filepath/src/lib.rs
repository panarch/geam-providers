#[geam::provider(package = "filepath", modules = [filepath])]
pub struct Component;

#[geam::module(path = "filepath")]
mod filepath {
    #[geam::function]
    fn is_windows() -> bool {
        cfg!(windows)
    }
}
