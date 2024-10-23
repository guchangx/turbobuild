
async fn unpackage<'a>(path: &str, content: std::borrow::Cow<'a, [u8]>) {

    //std::borrow::Cow<'a, [u8]>
    
    let cursor = std::io::Cursor::new(content);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();

    let path = common::utils::get_working_path("".to_string()).unwrap();
    zip.extract(path).unwrap();

}

