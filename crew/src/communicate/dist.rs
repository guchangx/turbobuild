use crate::communicate::packager;

struct Dist {

}

impl Dist {

    fn new() -> Self {
        return Self{

        }
    }

    pub fn sync() {
        tokio::spawn(async {
            let packager = self::packager::Packager::default();
            packager.toolchain().await;
        }
        );
    }

}

