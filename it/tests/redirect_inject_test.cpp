#include <windows.h>
#include <winternl.h>
#include <iostream>
#include <vector>


typedef NTSTATUS(NTAPI* PfnNtQueryDirectoryFile)(
    HANDLE FileHandle,
    HANDLE Event,
    PIO_APC_ROUTINE ApcRoutine,
    PVOID ApcContext,
    PIO_STATUS_BLOCK IoStatusBlock,
    PVOID FileInformation,
    ULONG Length,
    FILE_INFORMATION_CLASS FileInformationClass,
    BOOLEAN ReturnSingleEntry,
    PUNICODE_STRING FileName,
    BOOLEAN RestartScan
    );


typedef struct _FILE_DIRECTORY_INFORMATION {
    ULONG         NextEntryOffset;
    ULONG         FileIndex;
    LARGE_INTEGER CreationTime;
    LARGE_INTEGER LastAccessTime;
    LARGE_INTEGER LastWriteTime;
    LARGE_INTEGER ChangeTime;
    LARGE_INTEGER EndOfFile;
    LARGE_INTEGER AllocationSize;
    ULONG         FileAttributes;
    ULONG         FileNameLength;
    WCHAR         FileName[1];
} FILE_DIRECTORY_INFORMATION, * PFILE_DIRECTORY_INFORMATION;

int wmain(int argc, wchar_t** argv) {
    std::cout << "redirect_inject_test.exe running..." << std::endl;

    if (argc < 2) return -1;
    
    const wchar_t* dirPath = argv[1];

    HMODULE hNtdll = GetModuleHandleW(L"ntdll.dll");
    if (!hNtdll) return -1;

    auto NtQueryDirectoryFile = (PfnNtQueryDirectoryFile)GetProcAddress(hNtdll, "NtQueryDirectoryFile");
    if (!NtQueryDirectoryFile) return -1;


    HANDLE hDir = CreateFileW(
        dirPath,
        FILE_LIST_DIRECTORY | SYNCHRONIZE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        NULL,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS,
        NULL
    );

    if (hDir == INVALID_HANDLE_VALUE) {
        std::cerr << "Open Directory Failed: " << GetLastError() << " " << *dirPath << std::endl;
        return -1;
    }

    const ULONG bufferSize = 1024 * 64;
    std::vector<BYTE> buffer(bufferSize);
    IO_STATUS_BLOCK ioStatus;


    NTSTATUS status = NtQueryDirectoryFile(
        hDir, NULL, NULL, NULL, &ioStatus,
        buffer.data(), bufferSize,
        (FILE_INFORMATION_CLASS)1,
        FALSE, NULL, TRUE
    );

    if (status >= 0) {
        auto pInfo = reinterpret_cast<PFILE_DIRECTORY_INFORMATION>(buffer.data());
        while (true) {

            std::wstring fileName(pInfo->FileName, pInfo->FileNameLength / sizeof(WCHAR));
            std::wcout << L"Found: " << fileName << std::endl;


            if (pInfo->NextEntryOffset == 0) break;
            pInfo = reinterpret_cast<PFILE_DIRECTORY_INFORMATION>((BYTE*)pInfo + pInfo->NextEntryOffset);
        }
    }
    else {
        std::cerr << "NtQueryDirectoryFile Failed, Status: " << std::hex << status << std::endl;
    }

    CloseHandle(hDir);
    return 0;
}