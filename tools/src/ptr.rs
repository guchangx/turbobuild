
pub struct HandleBox {
    handle: winapi::shared::ntdef::HANDLE,
}
 
impl HandleBox {
    pub fn new(h: winapi::shared::ntdef::HANDLE) -> Self {
        Self { handle: h }
    }
 
    pub fn get(&self) -> &winapi::shared::ntdef::HANDLE {
        &self.handle
    }

    #[allow(dead_code)]
    fn clone(&self) -> Self {
        Self { handle: self.handle.clone() }
    }
}
 
unsafe impl Send for HandleBox {}
unsafe impl Sync for HandleBox {}