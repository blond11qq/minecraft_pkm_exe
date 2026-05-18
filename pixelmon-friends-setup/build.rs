use std::path::Path;

#[cfg(target_os = "windows")]
const WINDOWS_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="0.1.0.0" processorArchitecture="*" name="PixelmonFriends" type="win32" />
  <description>Pixelmon Friends Minecraft mod installer</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 7 -->
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}" />
      <!-- Windows 8 -->
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}" />
      <!-- Windows 8.1 -->
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}" />
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
    </application>
  </compatibility>
</assembly>
"#;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_manifest(WINDOWS_MANIFEST)
            .set("FileDescription", "Pixelmon Friends")
            .set("ProductName", "Pixelmon Friends")
            .set("OriginalFilename", "PixelmonFriends.exe");

        if Path::new("assets/icon.ico").exists() {
            res.set_icon("assets/icon.ico");
        }

        res.compile()
            .expect("failed to embed Windows application resources");
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = Path::new("assets/icon.ico");
    }
}
