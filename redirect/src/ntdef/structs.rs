#![allow(non_camel_case_types)]

#[repr(C, align(4))]
pub struct _FILE_NAMES_INFORMATION {
    pub NextEntryOffset: super::types::ULONG,
    pub FileIndex: super::types::ULONG,
    pub FileNameLength: super::types::ULONG,
    pub FileName: u16,   // this is the string itself
}

pub type PFILE_NAMES_INFORMATION = *mut _FILE_NAMES_INFORMATION;

// todo: make these structs?
pub type PFILE_OBJECT = super::types::PVOID;
pub type PDEVICE_OBJECT = super::types::PVOID;
pub type PMDL = super::types::PVOID;
pub type PEPROCESS = super::types::PVOID;
pub type PRKPROCESS = super::types::PVOID;
pub type PKPROCESS = super::types::PVOID;


#[repr(C)]
pub struct UNICODE_STRING {
    pub     Length:         super::types::USHORT,
    pub     MaximumLength:  super::types::USHORT,
    pub     Buffer:         super::types::PWSTR,
}

pub type PUNICODE_STRING = *mut UNICODE_STRING;

#[repr(C, packed)]
pub struct LARGE_INTEGER {
    pub QuadPart: super::types::UINT64,
    //pub     LowPart:        super::types::DWORD,
    //pub     HighPart:       super::types::DWORD,
}

pub type PLARGE_INTEGER = *mut LARGE_INTEGER;

#[repr(C)]
pub struct IO_STATUS_BLOCK {
    pub     Status:         super::types::NTSTATUS,   // union: Pointer: super::types::PVOID,
    pub     Information:    super::types::PVOID,
}

pub type PIO_STATUS_BLOCK = *mut IO_STATUS_BLOCK;

/*
1: kd> dt nt!_OBJECT_ATTRIBUTES
   +0x000 Length           : Uint4B
   +0x008 RootDirectory    : Ptr64 Void
   +0x010 ObjectName       : Ptr64 _UNICODE_STRING
   +0x018 Attributes       : Uint4B
   +0x020 SecurityDescriptor : Ptr64 Void
   +0x028 SecurityQualityOfService : Ptr64 Void
*/
#[repr(C)]
pub struct OBJECT_ATTRIBUTES {
    pub     Length:                     super::types::ULONG,
    pub     RootDirectory:              super::types::HANDLE,
    pub     ObjectName:                 super::structs::PUNICODE_STRING,
    pub     Attributes:                 super::types::ULONG,
    pub     SecurityDescriptor:         super::types::PVOID,
    pub     SecurityQualityOfService:   super::types::PVOID,
}


pub type POBJECT_ATTRIBUTES = *mut OBJECT_ATTRIBUTES;

#[repr(C, packed)]
pub struct LIST_ENTRY {
    pub Flink:  super::types::PVOID,
    pub Blink:  super::types::PVOID,
}

#[repr(C, packed)]
pub struct KEVENT {
    pub Lock:           super::types::LONG,
    pub SignalState:    super::types::LONG,
    pub WaitListHead:   super::structs::LIST_ENTRY,
}

pub type PKEVENT = *mut KEVENT;

// TDI
#[repr(C, packed)]
pub struct TDI_REQUEST_KERNEL_ASSOCIATE { 
    pub AddressHandle:  super::types::HANDLE,
}

pub type PTDI_REQUEST_KERNEL_ASSOCIATE = *mut TDI_REQUEST_KERNEL_ASSOCIATE;

#[repr(C)]
pub struct TDI_REQUEST_KERNEL_SET_EVENT {
    pub EventType:      super::types::LONG,
    pub EventHandler:   super::types::PVOID,
    pub EventContext:   super::types::PVOID,
}

pub type PTDI_REQUEST_KERNEL_SET_EVENT = *mut TDI_REQUEST_KERNEL_SET_EVENT;

#[repr(C)]
pub struct TDI_CONNECTION_INFORMATION {
    pub UserDataLength:         super::types::LONG,
    pub UserData:               super::types::PVOID,
    pub OptionsLength:          super::types::LONG,
    pub Options:                super::types::PVOID,
    pub RemoteAddressLength:    super::types::LONG,
    pub RemoteAddress:          super::types::PVOID,
}

pub type PTDI_CONNECTION_INFORMATION = *mut TDI_CONNECTION_INFORMATION;

#[repr(C)]
pub struct TDI_REQUEST_KERNEL {
    pub RequestFlags:                       super::types::ULONG_PTR,
    pub RequestConnectionInformation:       super::structs::PTDI_CONNECTION_INFORMATION,
    pub ReturnConnectionInformation:        super::structs::PTDI_CONNECTION_INFORMATION,
    pub RequestSpecific:                    super::types::PVOID,
}

pub type PTDI_REQUEST_KERNEL = *mut TDI_REQUEST_KERNEL;

#[repr(C, packed)]
pub struct TDI_REQUEST_KERNEL_SEND {
    pub SendLength: super::types::ULONG,
    pub SendFlags: super::types::ULONG,
}

pub type PTDI_REQUEST_KERNEL_SEND = *mut TDI_REQUEST_KERNEL_SEND;

#[cfg(target_arch="x86_64")]
#[repr(C, packed)]
pub struct IRP {
    pub Type: super::types::USHORT,
    pub Size: super::types::USHORT,
    pub Padding_0x4_0x8:  [u8; 4],
    pub MdlAddress: super::structs::PMDL,
    // and more...
}

pub type PIRP = *mut super::structs::IRP;

#[cfg(target_arch="x86_64")]
#[repr(C, packed)]
pub struct IO_STACK_LOCATION {
    pub MajorFunction:      super::types::UCHAR,
    pub MinorFunction:      super::types::UCHAR,
    pub Flags:              super::types::UCHAR,
    pub Control:            super::types::UCHAR,
    pub Padding_0x4_0x8:    [u8; 0x4],
    pub Parameters:         [super::types::BYTE; 0x20],
    pub DeviceObject:       super::structs::PDEVICE_OBJECT,
    pub FileObject:         super::structs::PFILE_OBJECT,
    pub CompletionRoutine:  super::types::PVOID,
    pub Context:            super::types::PVOID,
}

pub type PIO_STACK_LOCATION = *mut IO_STACK_LOCATION;

#[repr(C, packed)]
pub struct FILE_FULL_EA_INFORMATION {
    pub NextEntryOffset:        super::types::ULONG,
    pub Flags:                  super::types::UCHAR,
    pub EaNameLength:           super::types::UCHAR,
    pub EaValueLength:          super::types::USHORT,
    // pub EaName: [u8; 1],
} 

pub type PFILE_FULL_EA_INFORMATION = *mut FILE_FULL_EA_INFORMATION;


#[repr(C, packed)]
pub struct TDI_ADDRESS_IP {
    pub sin_port:   super::types::USHORT,
    pub in_addr:    super::types::ULONG,
    pub sin_zero:   [super::types::UCHAR; 8],
}

pub type PTDI_ADDRESS_IP = *mut TDI_ADDRESS_IP;

#[repr(C, packed)]
pub struct TA_ADDRESS {
    pub AddressLength:  super::types::USHORT,
    pub AddressType:    super::types::USHORT,
    pub Address:        super::structs::TDI_ADDRESS_IP,
}

pub type PTA_ADDRESS = *mut TA_ADDRESS;

#[repr(C, packed)]
pub struct TRANSPORT_ADDRESS {
    pub TAAddressCount:     super::types::LONG,
    pub Address:            [super::structs::TA_ADDRESS; 1],
}

pub type PTRANSPORT_ADDRESS = *mut TRANSPORT_ADDRESS;

#[repr(C)]
pub struct KAPC_STATE {
    pub ApcListHead:            [super::structs::LIST_ENTRY; 2],
    pub Process:                super::structs::PRKPROCESS,
    pub KernelApcInProgress:    super::types::UCHAR,
    pub KernelApcPending:       super::types::UCHAR,
    pub UserApcPending:         super::types::UCHAR,
}

pub type PKAPC_STATE = *mut KAPC_STATE;

#[repr(C)]
pub struct MEMORY_BASIC_INFORMATION {
    pub BaseAddress:            super::types::PVOID,
    pub AllocationBase:         super::types::PVOID,
    pub AllocationProtect:      super::types::ULONG,
    pub PartitionId:            super::types::USHORT,
    pub RegionSize:             super::types::SIZE_T,
    pub State:                  super::types::ULONG,
    pub Protect:                super::types::ULONG,
    pub Type:                   super::types::ULONG,
}

#[repr(C)]
pub struct RTL_OSVERSIONINFOW {
    pub dwOSVersionInfoSize:        super::types::ULONG,
    pub dwMajorVersion:             super::types::ULONG,
    pub dwMinorVersion:             super::types::ULONG,
    pub dwBuildNumber:              super::types::ULONG,
    pub dwPlatformId:               super::types::ULONG,
    pub szCSDVersion:               [super::types::UINT16; 128],    
}

pub type PRTL_OSVERSIONINFOW = *mut RTL_OSVERSIONINFOW;

#[cfg(target_arch="x86_64")]
#[repr(C, packed)]
pub struct LDR_DATA_TABLE_ENTRY {
    pub InLoadOrderLinks:               super::structs::LIST_ENTRY,
    pub InMemoryOrderLinks:             super::structs::LIST_ENTRY,
    pub InInitializationOrderLinks:     super::structs::LIST_ENTRY,
    pub DllBase:                        super::types::PVOID,
    pub EntryPoint:                     super::types::PVOID,
    pub SizeOfImage:                    super::types::ULONG,
    pub Padding_0x44_0x48:              [super::types::BYTE; 4],
    pub FullDllName:                    super::structs::UNICODE_STRING,
    pub BaseDllName:                    super::structs::UNICODE_STRING,
    /* ...etc... */
}

pub type PLDR_DATA_TABLE_ENTRY = *mut LDR_DATA_TABLE_ENTRY;

#[repr(C, packed)]
pub struct PEB_LDR_DATA {
    pub Length:                     super::types::ULONG,
    pub Initialized:                super::types::ULONG,
    pub SsHandle:                   super::types::PVOID,
    pub InLoadOrderModuleList:      super::structs::LIST_ENTRY,
    /* ...etc... */
}

pub type PPEB_LDR_DATA = *mut PEB_LDR_DATA;

#[repr(C, packed)]
pub struct PEB {
    pub InheritedAddressSpace:      super::types::BOOLEAN,
    pub ReadImageFileExecOptions:   super::types::BOOLEAN,
    pub BeingDebugged:              super::types::BOOLEAN,
    pub SpareBool:                  super::types::BOOLEAN,
    pub Padding_0x4_0x8:            [super::types::BYTE; 4],
    pub Mutant:                     super::types::HANDLE,
    pub ImageBaseAddress:           super::types::PVOID,
    pub Ldr:                        super::structs::PPEB_LDR_DATA,
    /* ...etc... */
}

pub type PPEB = *mut PEB;

#[repr(C)]
pub struct PROCESS_BASIC_INFORMATION { 
    pub Reserved1:          super::types::PVOID,
    pub PebBaseAddress:     super::structs::PPEB,
    pub Reserved2:          [super::types::PVOID; 2],
    pub UniqueProcessId:    super::types::ULONG_PTR,
    pub Reserved3:          super::types::PVOID,
}