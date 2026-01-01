
pub struct HandleBox {
    handle: crate::win::Foundation::HANDLE,
}
 
impl HandleBox {
    pub fn new(h: crate::win::Foundation::HANDLE) -> Self {
        Self { handle: h }
    }
 
    pub fn get(&self) -> &crate::win::Foundation::HANDLE {
        &self.handle
    }
    
    pub fn clone(&self) -> Self {
        Self { handle: self.handle.clone() }
    }
}
 
unsafe impl Send for HandleBox {}
unsafe impl Sync for HandleBox {}