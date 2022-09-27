
#[derive(Clone)]
pub struct Dist{
    into: String,
}

impl Dist {
    pub fn init()-> Dist {
        Dist { into: "test".to_string() }
    }
}