use crate::communicate::packager;

struct Dist {

}

impl Dist {

    fn new() -> Self {
        return Self{

        }
    }

    pub fn sync() {
        let packager = self::packager::Packager::default();
            tokio::spawn(async move {
                packager.toolchain("toolchain path", "").await;
            }
        );
    }

}

