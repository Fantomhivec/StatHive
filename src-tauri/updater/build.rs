fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icons/updater.ico");
        res.set("ProductName", "StatHive Updater");
        res.set("FileDescription", "StatHive Updater");
        if let Err(e) = res.compile() {
            println!("cargo:warning=updater icon embed failed: {e}");
        }
    }
}
