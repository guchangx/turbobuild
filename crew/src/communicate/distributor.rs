
#[derive(Default, Clone)]
pub struct Distributor {
    restor: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::ResourceList>>,
}

impl Distributor {

    pub fn new(restor: std::sync::Arc<std::sync::Mutex::<crate::roster::crews::ResourceList>>) -> Self {
        return Self{
            restor,
        }
    }

    pub fn sync(&self) {

    }

    fn scheduler() {

    }
}

