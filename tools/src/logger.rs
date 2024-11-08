
static LOGGER: std::sync::Once = std::sync::Once::new();

pub fn init_logger() {

    let mut builder = env_logger::Builder::new();
    let logger = builder
    .format_timestamp_millis()
    .write_style(env_logger::WriteStyle::Always)
    .format_level(true)
    .format_target(true)
    .filter(Some("captain"), log::LevelFilter::Trace)
    .filter(Some("crew"), log::LevelFilter::Trace)
    .filter(Some("cocrew"), log::LevelFilter::Trace)
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

pub fn init_once_logger() {
    LOGGER.call_once(|| {
        init_logger();
    });
}