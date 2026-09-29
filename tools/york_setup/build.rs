fn main() {
    #[cfg(windows)]
    {
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
        let ver = env!("CARGO_PKG_VERSION");
        let mut parts = ver.split('.');
        let maj = parts.next().unwrap_or("0");
        let min = parts.next().unwrap_or("5");
        let pat = parts.next().unwrap_or("0");
        let rc = out_dir.join("york_setup.rc");
        let ico = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("logo.ico");
        let rc_src = format!(
            r#"VS_VERSION_INFO VERSIONINFO
 FILEVERSION {maj},{min},{pat},0
 PRODUCTVERSION {maj},{min},{pat},0
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0x0L
 FILEOS 0x40004L
 FILETYPE 0x1L
 FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "CompanyName", "York Contributors"
            VALUE "FileDescription", "York Programming Language Setup"
            VALUE "FileVersion", "{ver}.0"
            VALUE "InternalName", "york-setup"
            VALUE "OriginalFilename", "york-setup.exe"
            VALUE "ProductName", "York"
            VALUE "ProductVersion", "{ver}.0"
            VALUE "LegalCopyright", "Copyright (c) 2026 York Contributors"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END

1 ICON "{ico}"
"#,
            maj = maj,
            min = min,
            pat = pat,
            ver = ver,
            ico = ico.to_string_lossy().replace('\\', "/"),
        );
        std::fs::write(&rc, rc_src).expect("write rc");
        embed_resource::compile(&rc, embed_resource::NONE);
    }
}
