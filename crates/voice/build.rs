//! Mémorise le dossier de build du profil (target/<profil>) : en développement,
//! le spike y retrouve les données d'espeak-ng générées par espeak-rs-sys.

fn main() {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    // OUT_DIR = target/<profil>/build/bullitik-voice-<hash>/out
    if let Some(profile_dir) = out.ancestors().nth(3) {
        println!(
            "cargo:rustc-env=BULLITIK_PROFILE_DIR={}",
            profile_dir.display()
        );
    }
    println!("cargo:rerun-if-changed=build.rs");
}
