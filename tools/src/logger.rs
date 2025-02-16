
static LOGGER: std::sync::Once = std::sync::Once::new();

pub fn init_logger(module: &str) {
    use std::io::Write;
    
    let mut builder = env_logger::Builder::new();
    let logger = builder
    .format(|buf, record| {
        writeln!(
            buf,
            "[{} {} {}:{}] {}",
            buf.timestamp_millis(),
            record.level(),
            record.file().unwrap_or("<unnamed>"),
            record.line().unwrap_or(0),
            record.args()
        )
    })
    .write_style(env_logger::WriteStyle::Always)
    .format_level(true)
    .format_target(true)
    //.filter(Some("captain"), log::LevelFilter::Trace)
    //.filter(Some("crew"), log::LevelFilter::Trace)
    //.filter(Some("cocrew"), log::LevelFilter::Trace)
    .filter(Some(module), log::LevelFilter::Trace)
    .target(env_logger::Target::Stdout)
    .try_init();

    match logger {
        Ok(_) => {
            log::info!("init logger sucessful");
        },
        Err(error) => {
            log::error!("init logger failed. {:?}", error);
        }
    }
}

pub fn init_once_logger() {
    LOGGER.call_once(|| {
        init_logger("");
    });
}