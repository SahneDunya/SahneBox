#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz
#![allow(dead_code)] // Henüz kullanılmayan kodlar için uyarı vermesin

// Sadece RISC-V 64-bit mimarisi için aktif et (SiFive S21 de bu kategoriye girer)
#[cfg(target_arch = "riscv64")]
pub mod arch {
    // Mimariye özel sistem çağrı numaraları (Sahne64 terminolojisi ile)
    pub const SYSCALL_MEMORY_ALLOCATE: u64 = 1;  // Bellek tahsis et
    pub const SYSCALL_MEMORY_RELEASE: u64 = 2;   // Bellek serbest bırak (Handle ile?) - Şimdilik adres/boyut ile
    pub const SYSCALL_TASK_EXIT: u64 = 4;        // Mevcut görevi sonlandır (veya ana iş parçacığını)
    pub const SYSCALL_RESOURCE_ACQUIRE: u64 = 5; // Bir kaynağa erişim tanıtıcısı (Handle) al
    pub const SYSCALL_RESOURCE_READ: u64 = 6;    // Kaynaktan oku (Handle ile)
    pub const SYSCALL_RESOURCE_WRITE: u64 = 7;   // Kaynağa yaz (Handle ile)
    pub const SYSCALL_RESOURCE_RELEASE: u64 = 8; // Kaynak tanıtıcısını serbest bırak
    pub const SYSCALL_TASK_SLEEP: u64 = 10;      // Görevi/iş parçacığını uyut
    pub const SYSCALL_LOCK_CREATE: u64 = 11;     // Kilit (Lock) oluştur
    pub const SYSCALL_LOCK_ACQUIRE: u64 = 12;    // Kilidi al (Bloklayabilir)
    pub const SYSCALL_LOCK_RELEASE: u64 = 13;    // Kilidi bırak
    pub const SYSCALL_THREAD_CREATE: u64 = 14;   // Yeni bir iş parçacığı (thread) oluştur
    pub const SYSCALL_THREAD_EXIT: u64 = 15;     // Mevcut iş parçacığını sonlandır
    pub const SYSCALL_GET_SYSTEM_TIME: u64 = 16; // Sistem saatini al
    pub const SYSCALL_SHARED_MEM_CREATE: u64 = 17; // Paylaşımlı bellek alanı oluştur (Handle döner)
    pub const SYSCALL_SHARED_MEM_MAP: u64 = 18;   // Paylaşımlı belleği adres alanına eşle (Handle ile)
    pub const SYSCALL_SHARED_MEM_UNMAP: u64 = 19; // Paylaşımlı bellek eşlemesini kaldır
    pub const SYSCALL_TASK_YIELD: u64 = 101;     // CPU'yu başka bir çalıştırılabilir iş parçacığına devret
}

// SiFive S21 gibi RISC-V 64 için arch modülü tanımlanmamışsa derleme hatası ver
#[cfg(not(target_arch = "riscv64"))]
compile_error!("This crate is intended for riscv64 architecture (e.g., SiFive S21).");


/// Sahne64 Kaynak Tanıtıcısı (Handle).
/// Kaynaklara (dosyalar, soketler, bellek bölgeleri vb.) erişmek için kullanılır.
/// Bu, Unix'teki file descriptor'ların yerine geçer ve daha soyut bir kavramdır.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)] // Bellekte sadece u64 olarak yer kaplar
pub struct Handle(u64);

impl Handle {
    /// Geçersiz veya boş bir Handle oluşturur.
    pub const fn invalid() -> Self {
        Handle(0) // Veya çekirdeğin belirlediği başka bir geçersiz değer
    }

    /// Handle'ın geçerli olup olmadığını kontrol eder.
    pub fn is_valid(&self) -> bool {
        self.0 != Self::invalid().0
    }

    /// Handle'ın içindeki ham değeri alır (dikkatli kullanılmalı!).
    pub(crate) fn raw(&self) -> u64 {
        self.0
    }
}

/// Sahne64 Görev (Task) Tanımlayıcısı.
/// Minimal API'de TaskId doğrudan kullanılmayabilir, ancak Handle ve Thread ID'leri için
/// altyapıda gerekebilir.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TaskId(u64);

impl TaskId {
    /// Geçersiz bir TaskId oluşturur.
    pub const fn invalid() -> Self {
        TaskId(0) // Veya çekirdeğin belirlediği başka bir geçersiz değer
    }

    /// TaskId'nin geçerli olup olmadığını kontrol eder.
    pub fn is_valid(&self) -> bool {
        self.0 != Self::invalid().0
    }

    /// TaskId'nin içindeki ham değeri alır (dikkatli kullanılmalı!).
    pub(crate) fn raw(&self) -> u64 {
        self.0
    }
}


// Sahne64 Hata Türleri (Minimal set için ilgili hataları tutalım)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SahneError {
    OutOfMemory,          // Yetersiz bellek
    InvalidAddress,       // Geçersiz bellek adresi
    InvalidParameter,     // Fonksiyona geçersiz parametre verildi
    ResourceNotFound,     // Belirtilen kaynak bulunamadı (örn. isimle ararken)
    PermissionDenied,     // İşlem için yetki yok
    ResourceBusy,         // Kaynak şu anda meşgul (örn. kilitli)
    Interrupted,          // İşlem bir sinyal veya başka bir olayla kesildi (sleep/acquire gibi bloklayanlarda)
    InvalidOperation,     // Kaynak üzerinde geçersiz işlem denendi (örn. okunamaz kaynağı okumak)
    NotSupported,         // İşlem veya özellik desteklenmiyor (Olmayan syscall'ı çağırmak gibi)
    UnknownSystemCall,    // Çekirdek bilinmeyen sistem çağrısı numarası aldı
    TaskCreationFailed,   // Yeni iş parçacığı (thread) oluşturulamadı
    InvalidHandle,        // Geçersiz veya süresi dolmuş Handle kullanıldı
    HandleLimitExceeded,  // Süreç başına düşen Handle limiti aşıldı
    NamingError,          // Kaynak isimlendirme ile ilgili hata
    // CommunicationError ve NoMessage kaldırıldı
    // Diğer Sahne64'e özel hata kodları burada olabilir
}

// Sistem çağrısı arayüzü (çekirdeğe geçiş mekanizması)
// RISC-V 64-bit için yaygın ABI "sysv64" veya "C" dir. Sahne64'ün kendi ABI'si olabilir.
// Şimdilik "C" kullanalım, çoğu 64-bit platformda işe yarar.
extern "C" {
    fn syscall(number: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64;
}

// Hata Kodu Çevirimi Yardımcı Fonksiyonu
// Çekirdekten dönen negatif sayıları SahneError'a çevirir.
// Sadece minimal API ile ilgili hatalar maplenmeli.
fn map_kernel_error(code: i64) -> SahneError {
    match code {
        -1 => SahneError::PermissionDenied,
        -2 => SahneError::ResourceNotFound,
        -3 => SahneError::TaskCreationFailed, // Thread creation might return this
        -4 => SahneError::Interrupted,
        -9 => SahneError::InvalidHandle,
        -11 => SahneError::ResourceBusy,
        -12 => SahneError::OutOfMemory,
        -13 => SahneError::PermissionDenied, // ACCES gibi
        -14 => SahneError::InvalidAddress,
        -17 => SahneError::NamingError,
        -22 => SahneError::InvalidParameter,
        -38 => SahneError::NotSupported,
        // CommunicationError ve NoMessage hatalarını kaldırıyoruz
        // ... diğer Sahne64'e özel minimal hata kodları ...
        _ => SahneError::UnknownSystemCall, // Bilinmeyen veya eşlenmemiş hata
    }
}


// Bellek yönetimi modülü
pub mod memory {
    use super::{SahneError, arch, syscall, map_kernel_error, Handle};

    /// Belirtilen boyutta bellek ayırır.
    /// Başarılı olursa, ayrılan belleğe işaretçi döner.
    pub fn allocate(size: usize) -> Result<*mut u8, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MEMORY_ALLOCATE, size as u64, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as *mut u8)
        }
    }

    /// Daha önce `allocate` ile ayrılmış bir belleği serbest bırakır.
    pub fn release(ptr: *mut u8, size: usize) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MEMORY_RELEASE, ptr as u64, size as u64, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    /// Belirtilen boyutta paylaşımlı bellek alanı oluşturur ve bir Handle döner.
    pub fn create_shared(size: usize) -> Result<Handle, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_SHARED_MEM_CREATE, size as u64, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(Handle(result as u64))
        }
    }

    /// Paylaşımlı bellek Handle'ını mevcut görevin adres alanına eşler.
    pub fn map_shared(handle: Handle, offset: usize, size: usize) -> Result<*mut u8, SahneError> {
          if !handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let result = unsafe {
            syscall(arch::SYSCALL_SHARED_MEM_MAP, handle.raw(), offset as u64, size as u64, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as *mut u8)
        }
    }

    /// Eşlenmiş paylaşımlı bellek alanını adres alanından kaldırır.
    pub fn unmap_shared(addr: *mut u8, size: usize) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_SHARED_MEM_UNMAP, addr as u64, size as u64, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }
}

// Görev (Task) ve İş Parçacığı (Thread) yönetimi modülü
// Minimal API'de tek bir ana görev (task) içinde iş parçacıkları (thread) varsayımı.
pub mod task {
    use super::{SahneError, arch, syscall, map_kernel_error}; // TaskId artık doğrudan kullanılmıyor

    /// Mevcut görevi (veya ana iş parçacığını) belirtilen çıkış koduyla sonlandırır. Bu fonksiyon geri dönmez.
    pub fn exit(code: i32) -> ! {
        unsafe {
            syscall(arch::SYSCALL_TASK_EXIT, code as u64, 0, 0, 0, 0);
        }
        // Syscall başarısız olsa bile (ki olmamalı), görevi sonlandırmak için döngü.
        loop { core::hint::spin_loop(); }
    }

    /// Mevcut görevi/iş parçacığını belirtilen milisaniye kadar uyutur.
    pub fn sleep(milliseconds: u64) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_TASK_SLEEP, milliseconds, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    /// Yeni bir iş parçacığı (thread) oluşturur.
    /// İş parçacıkları aynı görev adres alanını paylaşır.
    /// `entry_point`: Yeni iş parçacığının başlangıç fonksiyon adresi.
    /// `stack_size`: Yeni iş parçacığı için ayrılacak yığın boyutu.
    /// `arg`: Başlangıç fonksiyonuna geçirilecek argüman.
    /// Başarılı olursa, yeni iş parçacığının ID'sini (u64) döner.
    pub fn create_thread(entry_point: u64, stack_size: usize, arg: u64) -> Result<u64, SahneError> { // u64 -> Thread ID
        let result = unsafe {
            syscall(arch::SYSCALL_THREAD_CREATE, entry_point, stack_size as u64, arg, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as u64) // Thread ID
        }
    }

    /// Mevcut iş parçacığını sonlandırır. Bu fonksiyon geri dönmez.
    pub fn exit_thread(code: i32) -> ! {
        unsafe {
            syscall(arch::SYSCALL_THREAD_EXIT, code as u64, 0, 0, 0, 0);
        }
        loop { core::hint::spin_loop(); }
    }

    /// CPU'yu gönüllü olarak başka bir çalıştırılabilir iş parçacığına bırakır.
    pub fn yield_now() -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_TASK_YIELD, 0, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }
}

// Kaynak yönetimi modülü (Dosya sistemi yerine donanım/soyut kaynaklar)
pub mod resource {
    use super::{SahneError, arch, syscall, map_kernel_error, Handle};

    // Kaynak açma/edinme modları için Sahne64'e özgü bayraklar
    pub const MODE_READ: u32 = 1 << 0;    // Kaynaktan okuma yeteneği iste
    pub const MODE_WRITE: u32 = 1 << 1;   // Kaynağa yazma yeteneği iste
    pub const MODE_CREATE: u32 = 1 << 2;  // Kaynak yoksa oluşturulsun (dosya benzeri olabilir)
    pub const MODE_EXCLUSIVE: u32 = 1 << 3; // Kaynak zaten varsa hata ver (CREATE ile kullanılır)
    pub const MODE_TRUNCATE: u32 = 1 << 4; // Kaynak açılırken içeriğini sil (varsa ve yazma izni varsa)
    // ... Sahne64'e özel diğer modlar (örn. NonBlocking)

    /// Sahne64'e özgü bir kaynak adı veya tanımlayıcısı.
    /// Minimal durumda bu genellikle bir donanım isimlendirmesi veya basit bir stringdir.
    pub type ResourceId<'a> = &'a str;

    /// Belirtilen ID'ye sahip bir kaynağa erişim Handle'ı edinir.
    /// `id`: Kaynağı tanımlayan Sahne64'e özgü tanımlayıcı (string).
    /// `mode`: Kaynağa nasıl erişileceğini belirten bayraklar (MODE_*).
    pub fn acquire(id: ResourceId, mode: u32) -> Result<Handle, SahneError> {
        let id_ptr = id.as_ptr() as u64;
        let id_len = id.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_ACQUIRE, id_ptr, id_len, mode as u64, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(Handle(result as u64))
        }
    }

    /// Belirtilen Handle ile temsil edilen kaynaktan veri okur.
    /// Okunan byte sayısını döner.
    pub fn read(handle: Handle, buffer: &mut [u8]) -> Result<usize, SahneError> {
        if !handle.is_valid() {
            return Err(SahneError::InvalidHandle);
        }
        let buffer_ptr = buffer.as_mut_ptr() as u64;
        let buffer_len = buffer.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_READ, handle.raw(), buffer_ptr, buffer_len, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as usize)
        }
    }

    /// Belirtilen Handle ile temsil edilen kaynağa veri yazar.
    /// Yazılan byte sayısını döner.
    pub fn write(handle: Handle, buffer: &[u8]) -> Result<usize, SahneError> {
          if !handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let buffer_ptr = buffer.as_ptr() as u64;
        let buffer_len = buffer.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_WRITE, handle.raw(), buffer_ptr, buffer_len, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as usize)
        }
    }

    /// Belirtilen Handle'ı serbest bırakır, kaynağa erişimi sonlandırır.
    pub fn release(handle: Handle) -> Result<(), SahneError> {
          if !handle.is_valid() {
              return Err(SahneError::InvalidHandle); // Zaten geçersiz handle'ı bırakmaya çalışma
          }
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_RELEASE, handle.raw(), 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    // control fonksiyonu kaldırıldı
}

// Çekirdek ile zaman etkileşim modülü
pub mod kernel {
    use super::{SahneError, arch, syscall, map_kernel_error};

    // KERNEL_INFO_ tipleri kaldırıldı

    // get_info fonksiyonu kaldırıldı

    /// Sistem saatini (örneğin, epoch'tan beri geçen nanosaniye olarak) alır.
    pub fn get_time() -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_GET_SYSTEM_TIME, 0, 0, 0, 0, 0)
        };
          if result < 0 {
              Err(map_kernel_error(result))
          } else {
              Ok(result as u64)
          }
    }
}

// Senkronizasyon araçları modülü (Mutex -> Lock)
pub mod sync {
    use super::{SahneError, arch, syscall, map_kernel_error, Handle};

    /// Yeni bir kilit (Lock) kaynağı oluşturur ve bunun için bir Handle döner.
    /// Başlangıçta kilit serbesttir.
    pub fn lock_create() -> Result<Handle, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_LOCK_CREATE, 0, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(Handle(result as u64))
        }
    }

    /// Belirtilen Handle'a sahip kilidi almaya çalışır.
    /// Kilit başka bir thread/task tarafından tutuluyorsa, çağıran bloke olur.
    pub fn lock_acquire(lock_handle: Handle) -> Result<(), SahneError> {
          if !lock_handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let result = unsafe {
            syscall(arch::SYSCALL_LOCK_ACQUIRE, lock_handle.raw(), 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    /// Belirtilen Handle'a sahip kilidi serbest bırakır.
    /// Kilidin çağıran thread/task tarafından tutuluyor olması gerekir.
    pub fn lock_release(lock_handle: Handle) -> Result<(), SahneError> {
          if !lock_handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let result = unsafe {
            syscall(arch::SYSCALL_LOCK_RELEASE, lock_handle.raw(), 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }
}