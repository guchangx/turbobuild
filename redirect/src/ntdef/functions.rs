

pub type ZwAllocateVirtualMemory = extern "stdcall" fn(
    ProcessHandle:      super::types::HANDLE,
    BaseAddress:        *mut super::types::PVOID,
    ZeroBits:           super::types::ULONG_PTR,
    RegionSize:         super::types::PSIZE_T,
    AllocationType:     super::types::ULONG,
    Protect:            super::types::ULONG,
) -> super::types::NTSTATUS;


pub type ZwCreateIoCompletion = extern "stdcall" fn(
    IoCompletionHandleReturn:   super::types::PHANDLE,
    DesiredAccess:              super::types::ACCESS_MASK,
    ObjectAttributes:           super::structs::POBJECT_ATTRIBUTES,
    Flags:                      super::types::ULONG,
) -> super::types::NTSTATUS;


pub type ZwSetInformationWorkerFactory = extern "stdcall" fn(
    WorkerFactoryHandle:            super::types::HANDLE,
    WorkerFactoryInformationClass:  super::enums::WORKER_FACTORY_INFORMATION_CLASS,
    WorkerFactoryInformation:       super::types::PVOID,
    WorkerFactoryInformationLength: super::types::SIZE_T,
) -> super::types::NTSTATUS;


pub type ZwCreateWorkerFactory = extern "stdcall" fn(
    WorkerFactoryHandleReturn:  super::types::PHANDLE,
    DesiredAccess:              super::types::ACCESS_MASK,
    ObjectAttributes:           super::structs::POBJECT_ATTRIBUTES,
    CompletionPortHandle:       super::types::HANDLE,
    WorkerProcessHandle:        super::types::HANDLE,
    StartRoutine:               super::types::PVOID,
    StartParameter:             super::types::PVOID,
    MaxThreadCount:             super::types::ULONG,
    StackReserve:               super::types::SIZE_T,
    StackCommit:                super::types::SIZE_T,
) -> super::types::NTSTATUS;

// https://github.com/mic101/windows/blob/master/WRK-v1.2/base/ntos/ps/create.c#L966-L1014
pub type PspCreateProcess = extern "stdcall" fn(
    ProcessHandle:          super::types::PHANDLE,  // out param
    DesiredAccess:          super::types::ACCESS_MASK,
    ObjectAttributes:       super::structs::POBJECT_ATTRIBUTES, // optional but used for argv
    ParentProcess:          super::types::HANDLE, // optional
    Flags:                  super::types::ULONG,
    SectionHandle:          super::types::HANDLE, // optional but used for new process from scratch (not forked)
    DebugPort:              super::types::HANDLE, // optional
    ExceptionPort:          super::types::HANDLE, // optional
    JobMemberLevel:         super::types::ULONG,
) -> super::types::NTSTATUS;


pub type ZwCreateSection = extern "stdcall" fn(
    SectionHandle:          super::types::PHANDLE,
    DesiredAccess:          super::types::ACCESS_MASK,
    ObjectAttributes:       super::structs::POBJECT_ATTRIBUTES,
    MaximumSize:            super::structs::PLARGE_INTEGER,
    SectionPageProtection:  super::types::ULONG,
    AllocationAttributes:   super::types::ULONG,
    FileHandle:             super::types::HANDLE,
) -> super::types::NTSTATUS;


pub type ZwSetValueKey = extern "stdcall" fn(
    KeyHandle:              super::types::HANDLE,
    ValueName:              super::structs::PUNICODE_STRING,
    TitleIndex:             super::types::ULONG,
    Type:                   super::types::ULONG,
    Data:                   super::types::PVOID,
    DataSize:               super::types::ULONG,
) -> super::types::NTSTATUS;


pub type ZwCreateKey = extern "stdcall" fn(
    KeyHandle:              super::types::PHANDLE,
    DesiredAccess:          super::types::ACCESS_MASK,
    ObjectAttributes:       super::structs::POBJECT_ATTRIBUTES,
    TitleIndex:             super::types::ULONG,
    Class:                  super::structs::PUNICODE_STRING,
    CreateOptions:          super::types::ULONG,
    Disposition:            super::types::PULONG,
) -> super::types::NTSTATUS;


pub type ZwQueryValueKey = extern "stdcall" fn(
    KeyHandle:                  super::types::HANDLE,
    ValueName:                  super::structs::PUNICODE_STRING,
    KeyValueInformationClass:   super::types::KEY_VALUE_INFORMATION_CLASS,
    KeyValueInformation:        super::types::PVOID,
    Length:                     super::types::ULONG,
    ResultLength:               super::types::PULONG,
) -> super::types::NTSTATUS;


pub type ZwOpenKey = extern "stdcall" fn(
    KeyHandle:              super::types::PHANDLE,
    DesiredAccess:          super::types::ACCESS_MASK,
    ObjectAttributes:       super::structs::POBJECT_ATTRIBUTES,
) -> super::types::NTSTATUS;


pub type ZwReadFile = extern "stdcall" fn(
    FileHandle:             super::types::HANDLE,
    Event:                  super::types::HANDLE,
    ApcRoutine:             super::types::PVOID,
    ApcContext:             super::types::PVOID,
    IoStatusBlock:          super::structs::PIO_STATUS_BLOCK,
    Buffer:                 super::types::PVOID,
    Length:                 super::types::ULONG,
    ByteOffset:             super::structs::PLARGE_INTEGER,
    Key:                    super::types::PULONG,
) -> super::types::NTSTATUS;


pub type ZwWriteFile = extern "stdcall" fn(
    FileHandle:             super::types::HANDLE,
    Event:                  super::types::HANDLE,
    ApcRoutine:             super::types::PVOID,
    ApcContext:             super::types::PVOID,
    IoStatusBlock:          super::structs::PIO_STATUS_BLOCK,
    Buffer:                 super::types::PVOID,
    Length:                 super::types::ULONG,
    ByteOffset:             super::structs::PLARGE_INTEGER,
    Key:                    super::types::PULONG,
) -> super::types::NTSTATUS;


pub type PsTerminateSystemThread = extern "stdcall" fn(
    ExitStatus:             super::types::NTSTATUS,
) -> ();


pub type PsCreateSystemThread = extern "stdcall" fn(
    ThreadHandle:           super::types::PHANDLE,
    DesiredAccess:          super::types::ULONG,
    ObjectAttributes:       super::types::PVOID,
    ProcessHandle:          super::types::HANDLE,
    ClientId:               super::types::PVOID,
    StartRoutine:           super::types::PVOID,
    StartContext:           super::types::PVOID,
) -> super::types::NTSTATUS;


pub type KeInitializeThreadedDpc = extern "stdcall" fn(
    Dpc:                    super::types::PVOID,
    DeferredRoutine:        super::types::PVOID,
    DeferredContext:        super::types::PVOID,
) -> ();


pub type MmMapIoSpace = extern "stdcall" fn(
    PhysicalAddress:        super::types::UINT64,
    NumberOfBytes:          super::types::SIZE_T,
    CacheType:              super::types::UINT32,
) -> super::types::PVOID;


pub type MmAllocateContiguousMemory = extern "stdcall" fn(
    NumberOfBytes:              super::types::SIZE_T,
    HighestAcceptableAddress:   super::types::UINT64,
) -> super::types::PVOID;


pub type ExTryToAcquireFastMutex = extern "stdcall" fn(
    FastMutex:              super::types::PVOID,
) -> super::types::BOOLEAN;


pub type ExInitializeFastMutex = extern "stdcall" fn(
    FastMutex:              super::types::PVOID,
) -> ();


pub type KeSetTimerEx = extern "stdcall" fn(
    Timer:                  super::types::PVOID,
    DueTime:                super::types::LARGE_INTEGER,
    Period:                 super::types::LONG,
    Dpc:                    super::types::PVOID,
) -> super::types::BOOLEAN;


pub type KeInitializeDpc = extern "stdcall" fn(
    Dpc:                    super::types::PVOID,
    DeferredRoutine:        super::types::PVOID,
    DeferredContext:        super::types::PVOID,
) -> ();


pub type KeInitializeTimer = extern "stdcall" fn(
    Timer:                  super::types::PVOID,
) -> ();


pub type ZwClose = extern "stdcall" fn(
    Handle:                 super::types::HANDLE,
) -> ();


pub type ZwQueryDirectoryFile = extern "stdcall" fn(
    FileHandle:             super::types::HANDLE,
    Event:                  super::types::HANDLE,
    ApcRoutine:             super::types::PVOID,
    ApcContext:             super::types::PVOID,
    IoStatusBlock:          super::structs::PIO_STATUS_BLOCK,
    FileInformation:        super::types::PVOID,
    Length:                 super::types::ULONG,
    FileInformationClass:   super::types::FILE_INFORMATION_CLASS,
    ReturnSingleEntry:      super::types::BOOLEAN,
    FileName:               super::structs::PUNICODE_STRING,
    RestartScan:            super::types::BOOLEAN
) -> super::types::NTSTATUS;

pub type KeRaiseIrqlToDpcLevel = extern "stdcall" fn() -> super::types::KIRQL;

pub type KeLowerIrql = extern "stdcall" fn(
    NewIrql:    super::types::KIRQL
) -> ();


pub type ExAllocatePool = extern "stdcall" fn(
    pool_type:  super::enums::POOL_TYPE,
    size:       super::types::SIZE_T,
) -> super::types::PVOID;

pub type ExFreePoolWithTag = extern "stdcall" fn(
    Buffer:     super::types::PVOID,
    Tag:        super::types::ULONG,
) -> ();

pub type ZwCreateFile = extern "stdcall" fn(
    FileHandle:         super::types::PHANDLE,
    AccessMask:         super::types::ACCESS_MASK,
    ObjectAttributes:   super::structs::POBJECT_ATTRIBUTES,
    IoStatusBlock:      super::structs::PIO_STATUS_BLOCK,
    AllocationSize:     super::structs::PLARGE_INTEGER,
    FileAttributes:     super::types::ULONG,
    ShareAccess:        super::types::ULONG,
    CreateDisposition:  super::types::ULONG,
    CreateOptions:      super::types::ULONG,
    EaBuffer:           super::types::PVOID,
    EaLength:           super::types::ULONG,
) -> super::types::NTSTATUS;

pub type ObReferenceObjectByHandle = extern "stdcall" fn(
    Handle:             super::types::HANDLE,
    AccessMask:         super::types::ACCESS_MASK,
    ObjectType:         super::types::PVOID, // POBJECT_TYPE,
    AccessMode:         super::enums::KPROCESSOR_MODE,
    Object:             *mut super::types::PVOID, // PVOID*,
    HandleInformation:  super::types::PVOID, // POBJECT_HANDLE_INFORMATION,
) -> super::types::NTSTATUS;

pub type IoBuildDeviceIoControlRequest = extern "stdcall" fn(
    IoControlCode:              super::types::ULONG,
    DeviceObject:               super::structs::PDEVICE_OBJECT,
    InputBuffer:                super::types::PVOID,
    InputBufferLength:          super::types::ULONG,
    OutputBuffer:               super::types::PVOID,
    OutputBufferLength:         super::types::ULONG,
    InternalDeviceIoControl:    super::types::BOOLEAN,
    Event:                      super::structs::PKEVENT,
    IoStatusBlock:              super::structs::PIO_STATUS_BLOCK,
) -> super::structs::PIRP;

pub type IoGetRelatedDeviceObject = extern "stdcall" fn(
    FileObject: super::structs::PFILE_OBJECT,
) -> super::structs::PDEVICE_OBJECT;

pub type IofCallDriver = extern "fastcall" fn(
    DeviceObject:   super::structs::PDEVICE_OBJECT,
    Irp:            super::structs::PIRP,
) -> super::types::NTSTATUS;

pub type KeInitializeEvent = extern "stdcall" fn(
    Event:      super::structs::PKEVENT,
    Type:       super::enums::EVENT_TYPE,
    State:      super::types::BOOLEAN,
) -> ();

pub type KeWaitForSingleObject = extern "stdcall" fn(
    Object:         super::types::PVOID,
    WaitReason:     super::enums::KWAIT_REASON,
    WaitMode:       super::enums::KPROCESSOR_MODE,
    Alertable:      super::types::BOOLEAN,
    Timeout:        super::structs::PLARGE_INTEGER,
) -> ();

pub type IoAllocateMdl = extern "stdcall" fn(
    VirtualAddress:     super::types::PVOID,
    Length:             super::types::ULONG,
    SecondaryBuffer:    super::types::BOOLEAN,
    ChargeQuote:        super::types::BOOLEAN,
    Irp:                super::structs::PIRP,
) -> super::structs::PMDL;

pub type IoFreeMdl = extern "stdcall" fn(
    Mdl:    super::structs::PMDL,
) -> ();

pub type MmBuildMdlForNonPagedPool = extern "stdcall" fn(
    Mdl:    super::structs::PMDL,
) -> ();

pub type MmProbeAndLockPages = extern "stdcall" fn(
    Mdl:        super::structs::PMDL,
    AccessMove: super::enums::KPROCESSOR_MODE,
    Operation:  super::enums::LOCK_OPERATION,
) -> ();

pub type MmSecureVirtualMemory = extern "stdcall" fn(
    Address:        super::types::PVOID,
    Size:           super::types::SIZE_T,
    ProbeMode:      super::types::ULONG,
) -> super::types::HANDLE;


pub type MmUnsecureVirtualMemory = extern "stdcall" fn(
    SecureHandle:        super::types::HANDLE,
) -> ();

pub type PsGetProcessImageFileName = extern "stdcall" fn(
    process:    super::structs::PEPROCESS,
) -> super::types::PCHAR;

pub type PsLookupProcessByProcessId  = extern "stdcall" fn(
    ProcessId:      super::types::HANDLE,
    Process:        *mut super::structs::PEPROCESS,
) -> super::types::NTSTATUS;

pub type ObfDereferenceObject = extern "fastcall" fn(
    Object:     super::types::PVOID,
) -> super::types::ULONG_PTR;

pub type KeStackAttachProcess = extern "stdcall" fn(
    Process:     super::structs::PRKPROCESS,
    ApcState:    super::structs::PKAPC_STATE,
) -> ();

pub type KeUnstackDetachProcess = extern "stdcall" fn(
    ApcState:   super::structs::PKAPC_STATE,
) -> ();

pub type ZwQueryVirtualMemory = extern "stdcall" fn(
    ProcessHandle:              super::types::HANDLE,
    BaseAddress:                super::types::PVOID,
    MemoryInformationClass:     super::enums::MEMORY_INFORMATION_CLASS,
    MemoryInformation:          super::types::PVOID,
    MemoryInformationLength:    super::types::SIZE_T,
    ReturnLength:               super::types::PSIZE_T,
) -> super::types::NTSTATUS;

pub type RtlGetVersion = extern "stdcall" fn(
    lpVersionInformation: super::structs::PRTL_OSVERSIONINFOW,
) -> super::types::NTSTATUS;

pub type ZwQueryInformationProcess = extern "stdcall" fn(
    ProcessHandle:  super::types::HANDLE,
    ProcessInformationClass:    super::enums::PROCESSINFOCLASS,
    ProcessInformation:         super::types::PVOID,
    ProcessInformationLength:   super::types::ULONG,
    ReturnLength:               super::types::PULONG,
) -> super::types::NTSTATUS;