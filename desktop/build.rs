fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../assets/icon.ico");

        let kit_bins = [
            r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64",
            r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64",
        ];
        for dir in kit_bins {
            if std::path::Path::new(dir).join("rc.exe").exists() {
                res.set_toolkit_path(dir);
                break;
            }
        }

        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {}", e);
        }
    }

    slint_build::compile("src/ui/windows/ui/settings.slint")
        .expect("Failed to compile settings.slint");
}
