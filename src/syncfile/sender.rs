
pub struct Sender<'a>{
    client: &'a crate::network::client::NetworkClient,
}

impl<'a> Sender<'a> {
    pub fn new(client: &'a crate::network::client::NetworkClient) -> Self {
        Sender {
            client,
        }
    }

    pub fn sync_dir(&self, dir: &str, client: &crate::network::client::NetworkClient) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            self.sync_dir(entry.path().to_str().unwrap(), client);
                        }
                        else if file_type.is_file() {
                            self.sync_file(entry.path().to_str().unwrap());
                        }
                    } 
                    else {
    
                    }
                }
            }
        }
    }
    pub fn sync_file(&self, file: &str) {
        self.client.dist_file_sync("syncmsvc",file)
    }
    
    pub fn sync_tool_chain(&self, path: &str) {
        let mut path = std::path::PathBuf::from(path);

        //C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.33.31629\bin\Hostx64\x64\cl.exe
        //msvc bin
        self.sync_file(path.to_str().unwrap());

        //msvc include 
        for _ in 0..4 {
            path.pop();
        }
        
        self.sync_dir(path.to_str().unwrap(), self.client);
    }

}



