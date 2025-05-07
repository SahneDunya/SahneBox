// main_kernel/tools/build.rs
// SahneBox Kernel Build Script

use std::env;

fn main() {
    // 1. Get the target architecture triplet
    let target = env::var("TARGET").expect("TARGET environment variable not set");
    println!("cargo:warning(Building for target: {})", target); // Log the target


    // 2. Create a cc::Build instance
    let mut builder = cc::Build::new();

    // Configure the builder for Assembly files
    builder
        .target(&target) // Set the target triple
        .flag("-c")      // Compile only, do not link
        .flag("-nostdinc") // Do not search for standard include directories (Assembly typically doesn't need them)
        .flag("-nostdlib"); // Do not link the standard library (not relevant for pure Assembly compilation, but good practice)

    // Add architecture-specific flags if needed (usually handled by the target triplet)
     builder.flag("-march=rv64gc")
     builder.flag("-mabi=lp64d")


    // 3. Add the Assembly source files
    // Relative paths from the kernel crate root (main_kernel/)
    builder.file("boot/head.S");
    builder.file("boot/boot.S");
    // TODO: Add other Assembly files if they exist, e.g.:
     builder.file("traps/trap_entry.S"); // If trap handler has Assembly stub
     builder.file("mm/page.S");         // If paging setup uses Assembly


    // 4. Compile the Assembly files
    // cc::Build::compile compiles the sources and tells Cargo about the resulting object files
    // The string argument is a name for the compilation target, often used as a library name
    // if archiving is enabled. For just .o files, it's less critical but still used.
    println!("cargo:warning(Compiling Assembly files...)");
    builder.compile("boot_assembly"); // Outputs boot_assembly.a or individual .o files depending on setup


    // 5. Tell Cargo to re-run the script if source files change
    println!("cargo:rerun-if-changed=boot/head.S");
    println!("cargo:rerun-if-changed=boot/boot.S");
    // TODO: Add other Assembly files here too if they were added above
     println!("cargo:rerun-if-changed=traps/trap_entry.S");
     println!("cargo:rerun-if-changed=mm/page.S");


    // TODO: If there are C source files, add them using builder.file("path/to/file.c")
    // The same builder can often handle C and Assembly.


    // Cargo should automatically link the compiled object files/archive produced by builder.compile()
    // because the cc crate prints the necessary cargo:rustc-link-search and cargo:rustc-link-lib lines.
    println!("cargo:warning(Build script finished)");
}