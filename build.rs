use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=data/resources.gresource.xml");
    println!("cargo:rerun-if-changed=data/icons");
    println!("cargo:rerun-if-changed=data/style.css");

    let status = Command::new("glib-compile-resources")
        .args(&[
            "--sourcedir=data",
            "--target=data/resources.gresource",
            "data/resources.gresource.xml",
        ])
        .status();

    if let Err(e) = status {
        eprintln!("Warning: Failed to execute glib-compile-resources: {}", e);
    }
}
