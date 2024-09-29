pub fn init_logger() {

    let mut builder = env_logger::Builder::new();
    let logger = builder
    .format_timestamp_millis()
    .write_style(env_logger::WriteStyle::Always)
    .format_level(true)
    .filter(Some("turbobuild"), log::LevelFilter::Trace)
    .target(env_logger::Target::Stdout)
    .try_init();

    match logger {
        Ok(_) => {
            println!("init logger sucessful");
        },
        Err(error) => {
            println!("init logger failed. {:?}", error);
        }
    }
}