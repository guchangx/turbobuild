#[cfg(test)]
mod integration_tests {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStrExt;

    #[test]
    fn redirect_inject_test() {

        let command: Vec<u16> = OsString::from("redirect_inject_test.exe")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let dll_path = "redirect64.dll";
        let dll_path_cstr = std::ffi::CString::new(dll_path).expect("Invalid redirect DLL path");

        unsafe {

            // Create pipes for stdout redirection
            let mut read_pipe: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
            let mut write_pipe: windows_sys::Win32::Foundation::HANDLE = std::ptr::null_mut();
            let mut sa: windows_sys::Win32::Security::SECURITY_ATTRIBUTES = std::mem::zeroed();
            sa.nLength = std::mem::size_of::<windows_sys::Win32::Security::SECURITY_ATTRIBUTES>() as u32;
            sa.bInheritHandle = windows_sys::Win32::Foundation::TRUE;
            sa.lpSecurityDescriptor = std::ptr::null_mut();

            let pipe_result = windows_sys::Win32::System::Pipes::CreatePipe(
                &mut read_pipe,
                &mut write_pipe,
                &mut sa,
                0,
            );
            assert_ne!(pipe_result, 0, "CreatePipe failed");

            let mut startup_info: cocrew::detours::detours::_STARTUPINFOW = std::mem::zeroed();
            startup_info.cb = std::mem::size_of::<cocrew::detours::detours::_STARTUPINFOW>() as u32;
            startup_info.dwFlags = windows_sys::Win32::System::Threading::STARTF_USESTDHANDLES;
            startup_info.hStdOutput = write_pipe;
            startup_info.hStdError = write_pipe;
            startup_info.hStdInput = windows_sys::Win32::System::Console::GetStdHandle(windows_sys::Win32::System::Console::STD_INPUT_HANDLE);

            // Initialize PROCESS_INFORMATION
            let mut process_info: cocrew::detours::detours::_PROCESS_INFORMATION = std::mem::zeroed();
            let dw_creation_flags = windows_sys::Win32::System::Threading::CREATE_DEFAULT_ERROR_MODE | windows_sys::Win32::System::Threading::CREATE_SUSPENDED | windows_sys::Win32::System::Threading::CREATE_UNICODE_ENVIRONMENT;

            let ret = cocrew::detours::detours::DetourCreateProcessWithDllExW(
                    std::ptr::null(),
                    command.as_ptr() as *mut u16, 
                    std::ptr::null_mut(), 
                    std::ptr::null_mut(), 
                    windows_sys::Win32::Foundation::TRUE,
                    dw_creation_flags, 
                    std::ptr::null_mut(), 
                    std::ptr::null(), 
                    &mut startup_info,
                    &mut process_info,
                    dll_path_cstr.as_ptr() as *const i8,
                    Option::None,
            );

            if ret == windows_sys::Win32::Foundation::TRUE {
                print!("DetourCreateProcessWithDllExW succeeded\n");
                windows_sys::Win32::System::Threading::ResumeThread(process_info.hThread);
                windows_sys::Win32::System::Threading::WaitForSingleObject(process_info.hProcess, windows_sys::Win32::System::Threading::INFINITE);    
            }
            else {
                print!("DetourCreateProcessWithDllExW failed\n");
            }

            // Close the write end of the pipe in the parent process
            windows_sys::Win32::Foundation::CloseHandle(write_pipe);
            
            // Read from the pipe
            print!("read dir list stdout\n");
            let mut buffer = Vec::new();
            let mut temp_buf = [0u8; 1024];
            loop {
                let mut bytes_read: u32 = 0;
                let read_result = windows_sys::Win32::Storage::FileSystem::ReadFile(
                    read_pipe,
                    temp_buf.as_mut_ptr(),
                    temp_buf.len() as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                );
                if read_result == windows_sys::Win32::Foundation::FALSE || bytes_read == 0 {
                    break;
                }
                buffer.extend_from_slice(&temp_buf[..bytes_read as usize]);
            }

            if !buffer.is_empty() {
                println!("dir list process stdout:\n{}", String::from_utf8_lossy(&buffer));
            }

            // Close the read pipe
            windows_sys::Win32::Foundation::CloseHandle(read_pipe);
        
            let mut code: u32 = 0;
            windows_sys::Win32::System::Threading::GetExitCodeProcess(process_info.hProcess, &mut code as _);
            assert_eq!(code, 0, "Injected process exited with error code {}", code);
            windows_sys::Win32::Foundation::CloseHandle(process_info.hProcess);
            windows_sys::Win32::Foundation::CloseHandle(process_info.hThread);
        }

    }
}