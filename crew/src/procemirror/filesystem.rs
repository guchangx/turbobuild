
pub fn route_file_system_operation(redirect: crate::communicate::package::pack::RemoteRedirect) {
    match redirect.api.as_str() {
        "" => {
        },
        _ => {
            log::warn!("Received unknown file system operation from remote redirect: {:?}", redirect);
        }
    }
}

fn redirect_nt_query_directory_file(dir: String) {

}