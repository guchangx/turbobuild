
#[derive(Clone)]
pub struct Dist{
    _into: String,
}

impl Dist {
    pub fn init()-> Dist {
        Dist { _into: "test".to_string() }
    }
}