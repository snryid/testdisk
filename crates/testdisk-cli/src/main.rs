fn main() {
    if let Err(error) = testdisk_cli::dispatch(std::env::args()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
