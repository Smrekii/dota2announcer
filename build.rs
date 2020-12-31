extern crate winres;

fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon_with_id("web/favicon.ico", "icon");
        res.set_language(0x0009); // English `0x0009`
        res.compile().unwrap();
    }
}
