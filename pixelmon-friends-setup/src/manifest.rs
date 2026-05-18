#[derive(Clone, Debug)]
pub struct Manifest {
    pub profile_id: String,
    pub profile_name: String,
    pub minecraft_version: String,
    pub neoforge_version: String,
    pub neoforge_version_id: String,
    pub pixelmon_version: String,
    pub pixelmon_modrinth_project: String,
    pub java_major: u32,
    pub ram_mb: u32,
    pub server_name: String,
    pub server_address: String,
    pub additional_mods: Vec<AdditionalMod>,
}

#[derive(Clone, Debug)]
pub struct AdditionalMod {
    pub name: String,
    pub modrinth_project: String,
    pub filename_prefixes: Vec<String>,
}

pub fn load_manifest() -> Manifest {
    Manifest {
        profile_id: "pixelmon-friends".to_string(),
        profile_name: "Pixelmon Friends".to_string(),
        minecraft_version: "1.21.1".to_string(),
        neoforge_version: "21.1.200".to_string(),
        neoforge_version_id: "neoforge-21.1.200".to_string(),
        pixelmon_version: "9.3.16".to_string(),
        pixelmon_modrinth_project: "pixelmon".to_string(),
        java_major: 21,
        ram_mb: 6144,
        server_name: "뭐해 포켓몬 모드 서버".to_string(),
        server_address: "34.64.32.34:25565".to_string(),
        additional_mods: vec![
            additional_mod("Sodium", "sodium", &["sodium-"]),
            additional_mod("ModernFix", "modernfix", &["modernfix-"]),
            additional_mod("FerriteCore", "ferrite-core", &["ferritecore-"]),
            additional_mod("Lithium", "lithium", &["lithium-"]),
            additional_mod("Entity Culling", "entityculling", &["entityculling-"]),
            additional_mod("ImmediatelyFast", "immediatelyfast", &["immediatelyfast-"]),
            additional_mod("Clumps", "clumps", &["clumps-"]),
        ],
    }
}

fn additional_mod(name: &str, modrinth_project: &str, filename_prefixes: &[&str]) -> AdditionalMod {
    AdditionalMod {
        name: name.to_string(),
        modrinth_project: modrinth_project.to_string(),
        filename_prefixes: filename_prefixes
            .iter()
            .map(|prefix| prefix.to_string())
            .collect(),
    }
}
