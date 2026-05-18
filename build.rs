fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_manifest_file("app.manifest");
        res.set("FileDescription", "Pixelmon Friends Client Tool");
        res.set("ProductName", "Pixelmon Friends Client");
        res.set("OriginalFilename", "PixelmonFriendsClient.exe");
        res.compile().expect("failed to embed Windows manifest");
    }
}
