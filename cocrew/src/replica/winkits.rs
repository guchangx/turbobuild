struct Model {
    winkits_version: String,
    replica_dir: String,
}

impl Model {
    fn new(winkits_version: String, replica_dir: String) -> Self {
        Model {
            winkits_version,
            replica_dir,
        }
    }
}