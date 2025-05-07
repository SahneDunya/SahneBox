// main_kernel/sys.rs
// Sistem Çağrısı İşleyicisi (Syscall Handler)
// sahne64.rs'de tanımlanan sistem çağrılarını işler.

use crate::printk;             // printk! makrosunu içeri aktar
use crate::traps::TrapFrame;    // TrapFrame yapısını içeri aktar (kaydedilmiş registerları içerir)
use crate::sahne64::arch;       // sahne64.rs'deki sistem çağrısı numaralarını içeri aktar
use crate::sahne64::SahneError; // sahne64.rs'deki hata enumunu içeri aktar

// TODO: İlgili çekirdek modüllerini içeri aktarın
use crate::exit;  // task::exit, thread::exit için
use crate::sched; // task::sleep, task::yield_now, thread::create için
use crate::mm;    // memory::allocate, memory::release, shared_mem_* için
use crate::resource_manager; // Resource syscallları için (şimdilik yok, sys.rs içinde placeholder)
use crate::sync_manager;     // Lock syscallları için (şimdilik yok, sys.rs içinde placeholder)
use crate::kernel_time;     // get_system_time için (şimdilik yok, sys.rs içinde placeholder)


// SahneError'ı ABI uyumlu negatif i64 hata koduna çeviren yardımcı fonksiyon (Kernel tarafı)
// Bu, sahne64::map_kernel_error fonksiyonunun tersine bir map'tir.
fn kernel_error_to_i64(err: SahneError) -> i64 {
    match err {
        SahneError::PermissionDenied => -1,
        SahneError::ResourceNotFound => -2,
        SahneError::TaskCreationFailed => -3, // Thread creation might return this
        SahneError::Interrupted => -4,
        SahneError::InvalidHandle => -9,
        SahneError::ResourceBusy => -11,
        SahneError::OutOfMemory => -12,
        SahneError::InvalidAddress => -14,
        SahneError::NamingError => -17,
        SahneError::InvalidParameter => -22,
        SahneError::NotSupported => -38,
        SahneError::InvalidOperation => -100, // Rastgele bir kod
        SahneError::HandleLimitExceeded => -101, // Rastgele bir kod
        SahneError::UnknownSystemCall => -102, // Rastgele bir kod
        // TODO: SahneError enumuna eklenen diğer hataları buraya mapleyin.
    }
}


// Sistem çağrısı işleyici fonksiyonu. traps.rs'den çağrılır.
// TrapFrame, sistem çağrısı numarasını (a7) ve argümanları (a0-a5) içerir.
// Başarılı durumda a0'a pozitif/sıfır, hata durumunda a0'a negatif hata kodu yazılır.
#[no_mangle] // traps::handle_trap fonksiyonundan çağrılabilmesi için isim bozulmamalı
pub extern "C" fn sys_call_handler(trap_frame: *mut TrapFrame) {
    // 'unsafe' çünkü raw pointer (trap_frame) kullanılıyor ve registerlara yazılıyor.
    unsafe {
        // Sistem çağrısı numarasını a7 registerından al
        let syscall_num = (*trap_frame).a7 as usize; // usize kullanmak match için daha uygun

        // Argümanları a0-a5 registerlarından al
        // Argüman sayısı sistem çağrısına göre değişir.
        let arg0 = (*trap_frame).a0;
        let arg1 = (*trap_frame).a1;
        let arg2 = (*trap_frame).a2;
        let arg3 = (*trap_frame).a3;
        let arg4 = (*trap_frame).a4;
        let arg5 = (*trap_frame).a5;

         printk!("Syscall #{} from MEPC {:#x}\n", syscall_num, (*trap_frame).mepc);

        // Sistem çağrısı numarasına göre ilgili kernel fonksiyonunu çağır
        // ve dönüş değerini trap_frame->a0'a yaz.
        // Syscall'dan dönen değerler genellikle i64 olarak yorumlanır (ABI gereği).
        let return_value: i64 = match syscall_num {
            arch::SYSCALL_MEMORY_ALLOCATE => {
                // allocate(size: usize) -> Result<*mut u8, SahneError>
                let size = arg0 as usize;
                match mm::sys_allocate(size) { // mm::sys_allocate kernel fonksiyonunuz olmalı
                    Ok(ptr) => ptr as i64, // Başarılı: Adresi i64 olarak döndür
                    Err(err) => kernel_error_to_i64(err), // Hata: Hata kodunu i64 olarak döndür
                }
            }
            arch::SYSCALL_MEMORY_RELEASE => {
                 release(ptr: *mut u8, size: usize) -> Result<(), SahneError>
                 let ptr = arg0 as *mut u8;
                 let size = arg1 as usize;
                 match mm::sys_deallocate(ptr, size) { // mm::sys_deallocate kernel fonksiyonunuz olmalı
                    Ok(()) => 0, // Başarılı: 0 döndür
                    Err(err) => kernel_error_to_i64(err), // Hata: Hata kodunu i64 olarak döndür
                 }
            }
            arch::SYSCALL_TASK_EXIT => {
                // exit(code: i32) -> ! (geri dönmez)
                let code = arg0 as i32;
                exit::sys_exit(code);
                // sys_exit schedule() çağırır ve bu görev sonlanır. Buraya asla ulaşılmaz.
                // Yine de, derleyiciyi memnun etmek için bir dönüş değeri sağlamamız gerekiyor,
                // ancak bu değer asla kullanılmayacaktır.
                // Genellikle bu tür syscall'lardan sonra bir sonsuz döngü konur.
                loop { core::hint::spin_loop(); } // Görev bitmezse burada dönsün
            }
            arch::SYSCALL_RESOURCE_ACQUIRE => {
                 acquire(id_ptr: u64, id_len: u64, mode: u32) -> Result<Handle, SahneError>
                let id_ptr = arg0 as *const u8;
                let id_len = arg1 as usize;
                let mode = arg2 as u32;

                // TODO: Kernel-internal resource management logic here or in resource_manager.rs
                // 1. id_ptr ve id_len ile gelen string'i doğrula (geçerli adres, çekirdek belleğinde değil vb.)
                // 2. Kaynak adını kernel'in tanıdığı kaynaklarla eşle (örn. "uart", "emmc0", "sdcard1", "display", "touchscreen", "refrigerator")
                // 3. İzinleri kontrol et (mode).
                // 4. İlgili sürücüyü veya kaynağı al/aç.
                // 5. Yeni bir Handle oluştur ve bunu geçerli göreve ata.
                // 6. Handle değerini veya hata kodunu döndür.

                // Şimdilik yer tutucu: Sadece "uart" kaynağını tanıyormuş gibi yapalım
                let resource_id_slice = core::slice::from_raw_parts(id_ptr, id_len);
                 let resource_name = core::str::from_utf8(resource_id_slice);

                 match resource_name {
                     Ok("uart") => {
                         // Başarılı bir Handle değeri döndür (örneğin 1)
                         // Gerçekte, çekirdek handle'ları yönetmeli.
                          1i64 // Varsayımsal "uart" handle'ı
                     }
                     Ok(_) | Err(_) => {
                          // Bilinmeyen kaynak veya geçersiz isim
                          kernel_error_to_i64(SahneError::ResourceNotFound)
                     }
                 }
                // Bu kısım ciddi implementasyon gerektirir.
            }
            arch::SYSCALL_RESOURCE_READ => {
                 // read(handle: u64, buf_ptr: u64, buf_len: u64) -> Result<usize, SahneError>
                 let handle_val = arg0;
                 let buf_ptr = arg1 as *mut u8;
                 let buf_len = arg2 as usize;

                 // TODO: Kernel-internal resource management logic
                 // 1. Handle'ı doğrula ve geçerli göreve ait mi kontrol et.
                 // 2. Handle'a karşılık gelen kaynağı (ve sürücüyü) bul.
                 // 3. Kaynağın okunabilir olup olmadığını kontrol et.
                 // 4. Sürücünün okuma fonksiyonunu çağır (örn. drivers::uart::getc veya drivers::emmc::read_block).
                 // 5. Okunan bayt sayısını veya hata kodunu döndür.

                 // Şimdilik yer tutucu: Sadece "uart" handle'ından okuyormuş gibi yapalım (varsayımsal handle 1)
                 if handle_val == 1 { // Varsayımsal "uart" handle'ı
                     // TODO: drivers::uart::getc() veya buffer okuma mantığını kullan
                     // drivers::uart::getc() tek bayt okur, syscall buffer bekler.
                     // Burası karmaşıklaşır. Basit polling okuma örneği:
                     let mut bytes_read = 0;
                     for i in 0..buf_len {
                         if let Some(byte) = crate::drivers::uart::getc() { // drivers::uart::getc()'i kullan
                             ptr::write_volatile(buf_ptr.add(i), byte);
                             bytes_read += 1;
                         } else {
                             // Veri yoksa hemen dön veya bir süre bekle (polling)
                             break; // Pollingde veri yoksa hemen dön
                         }
                     }
                     bytes_read as i64 // Okunan bayt sayısını döndür
                 } else {
                      kernel_error_to_i64(SahneError::InvalidHandle) // Geçersiz handle
                 }
                 // Bu kısım ciddi implementasyon gerektirir.
            }
            arch::SYSCALL_RESOURCE_WRITE => {
                 write(handle: u64, buf_ptr: u64, buf_len: u64) -> Result<usize, SahneError>
                 let handle_val = arg0;
                 let buf_ptr = arg1 as *const u8;
                 let buf_len = arg2 as usize;

                 // TODO: Kernel-internal resource management logic
                 // 1. Handle'ı doğrula ve geçerli göreve ait mi kontrol et.
                 // 2. Handle'a karşılık gelen kaynağı (ve sürücüyü) bul.
                 // 3. Kaynağın yazılabilir olup olmadığını kontrol et.
                 // 4. Sürücünün yazma fonksiyonunu çağır (örn. drivers::uart::putc veya drivers::emmc::write_block).
                 // 5. Yazılan bayt sayısını veya hata kodunu döndür.

                 // Şimdilik yer tutucu: Sadece "uart" handle'ına yazıyormuş gibi yapalım (varsayımsal handle 1)
                 if handle_val == 1 { // Varsayımsal "uart" handle'ı
                     // TODO: drivers::uart::putc() veya buffer yazma mantığını kullan
                     let slice = core::slice::from_raw_parts(buf_ptr, buf_len);
                     for &byte in slice {
                         crate::drivers::uart::putc(byte); // drivers::uart::putc() kullan
                     }
                     buf_len as i64 // Yazılan bayt sayısını döndür (basitlik)
                 } else {
                      kernel_error_to_i64(SahneError::InvalidHandle) // Geçersiz handle
                 }
                 // Bu kısım ciddi implementasyon gerektirir.
            }
            arch::SYSCALL_RESOURCE_RELEASE => {
                 release(handle: u64) -> Result<(), SahneError>
                let handle_val = arg0;

                 // TODO: Kernel-internal resource management logic
                 // 1. Handle'ı doğrula ve geçerli göreve ait mi kontrol et.
                 // 2. Handle'a karşılık gelen kaynağı serbest bırak / referans sayısını azalt.
                 // 3. Handle'ı geçersiz olarak işaretle.
                 // 4. Başarı veya hata kodunu döndür.

                 // Şimdilik yer tutucu: Sadece handle 1'i serbest bırakıyormuş gibi yapalım
                 if handle_val == 1 { // Varsayımsal "uart" handle'ı
                     // Gerçekte Handle tablosundan kaldırılmalı
                     0i64 // Başarılı
                 } else {
                     kernel_error_to_i64(SahneError::InvalidHandle) // Geçersiz handle
                 }
                // Bu kısım ciddi implementasyon gerektirir.
            }
            arch::SYSCALL_TASK_SLEEP => {
                  sleep(milliseconds: u64) -> Result<(), SahneError>
                 let milliseconds = arg0;
                 // TODO: scheduler ve timer ile etkileşime girerek görevi uyut
                 sched::sleep(milliseconds); // scheduler'da sleep fonksiyonunuz olmalı

                 // Uyku başarılıysa 0 döndür (kesilmezse)
                 // sched::sleep Result döndürmeli
                 0i64 // Başarılı varsayalım
                 // Bu kısım ciddi implementasyon gerektirir (Timer kesmeleri, Blocked görev yönetimi).
            }
            arch::SYSCALL_LOCK_CREATE => {
                 lock_create() -> Result<Handle, SahneError>
                // TODO: Kernel-internal lock management logic
                // 1. Yeni bir kernel mutex nesnesi oluştur.
                // 2. Bu mutex için bir Handle ata ve kaydet.
                // 3. Handle değerini döndür.

                // Şimdilik yer tutucu
                 kernel_error_to_i64(SahneError::NotSupported) // Henüz desteklenmiyor
                // Bu kısım ciddi implementasyon gerektirir.
            }
            arch::SYSCALL_LOCK_ACQUIRE => {
                 lock_acquire(lock_handle: u64) -> Result<(), SahneError>
                 let lock_handle_val = arg0;
                 // TODO: Kernel-internal lock management logic
                 // 1. Handle'ı doğrula ve Lock nesnesini bul.
                 // 2. Kilidi almaya çalış. Eğer kilitliyse, geçerli görevi Blocked durumuna al ve schedule().
                 // 3. Kilit başarıyla alındıysa 0 döndür.
                 // 4. Hata durumunda (örn. geçersiz handle, kesilme) hata kodu döndür.

                 // Şimdilik yer tutucu
                 kernel_error_to_i64(SahneError::NotSupported) // Henüz desteklenmiyor
                // Bu kısım ciddi implementasyon gerektirir.
            }
             arch::SYSCALL_LOCK_RELEASE => {
                 lock_release(lock_handle: u64) -> Result<(), SahneError>
                 let lock_handle_val = arg0;
                 // TODO: Kernel-internal lock management logic
                 // 1. Handle'ı doğrula ve Lock nesnesini bul.
                 // 2. Kilidi serbest bırak. Eğer kilit bekleyen görevler varsa, birini Runnable yap.
                 // 3. Başarı durumunda 0 döndür.
                 // 4. Hata durumunda (örn. geçersiz handle, kilit senin değil) hata kodu döndür.

                 // Şimdilik yer tutucu
                 kernel_error_to_i64(SahneError::NotSupported) // Henüz desteklenmiyor
                // Bu kısım ciddi implementasyon gerektirir.
            }
            arch::SYSCALL_THREAD_CREATE => {
                 // create_thread(entry_point: u64, stack_size: usize, arg: u64) -> Result<u64, SahneError>
                 let entry_point = arg0 as usize;
                 let stack_size = arg1 as usize;
                 let arg = arg2; // Argümanı entry point fonksiyonuna geçirmek gerekebilir

                 // TODO: scheduler'da thread yaratma fonksiyonunu çağır.
                 // sched::create_new_task(entry_point, stack_size, arg) gibi bir fonksiyon olmalı.
                 match sched::sys_create_thread(entry_point, stack_size, arg) { // scheduler'da sys_create_thread olmalı
                     Ok(thread_id) => thread_id as i64, // Başarılı: Thread ID'yi döndür
                     Err(err) => kernel_error_to_i64(err), // Hata: Hata kodunu döndür
                 }
                 // sched::sys_create_thread fonksiyonu Task::new'u çağırır ve task listesine ekler.
            }
             arch::SYSCALL_THREAD_EXIT => {
                // exit_thread(code: i32) -> ! (geri dönmez)
                let code = arg0 as i32;
                exit::sys_exit(code); // Şu an Task Exit ile aynı fonksiyonu çağırabiliriz
                 /// sys_exit schedule() çağırır ve bu thread sonlanır. Buraya asla ulaşılmaz.
                loop { core::hint::spin_loop(); }
            }
            arch::SYSCALL_GET_SYSTEM_TIME => {
                  get_time() -> Result<u64, SahneError>
                 // TODO: Kernelin zaman kaynağından (timer driver) mevcut zamanı al.
                 // kernel_time::get_time_nanos() gibi bir fonksiyon olmalı.

                 // Şimdilik yer tutucu: Sabit bir değer veya basit bir sayıcı döndürelim.
                  printk!("WARN: get_system_time implemente edilmedi, 0 döndürülüyor.\n");
                 0i64 // Başarılı varsayalım, zaman 0

                 // Gerçek implementasyon:
                  match kernel_time::get_time_nanos() {
                     Ok(time) => time as i64,
                     Err(err) => kernel_error_to_i64(err),
                  }
                 // Bu kısım ciddi implementasyon gerektirir (Timer donanımı ve zaman takibi).
            }
             arch::SYSCALL_SHARED_MEM_CREATE => {
                  create_shared(size: usize) -> Result<Handle, SahneError>
                 let size = arg0 as usize;
                 // TODO: Kernel-internal shared memory management logic (MMU/Paging ile ilgili)
                 // 1. Shared bellek alanı için fiziksel bellek tahsis et.
                 // 2. Bu alan için bir SharedMemory nesnesi/tanıtıcısı oluştur.
                 // 3. Bir Handle ata ve kaydet.
                 // 4. Handle değerini döndür.

                 // Şimdilik yer tutucu
                 kernel_error_to_i64(SahneError::NotSupported) // Henüz desteklenmiyor
                 // Bu kısım ciddi implementasyon gerektirir (MMU/Paging, bellek tahsisi).
            }
             arch::SYSCALL_SHARED_MEM_MAP => {
                  map_shared(handle: u64, offset: usize, size: usize) -> Result<*mut u8, SahneError>
                 let handle_val = arg0;
                 let offset = arg1 as usize;
                 let size = arg2 as usize;
                 // TODO: Kernel-internal shared memory management logic (MMU/Paging ile ilgili)
                 // 1. Handle'ı doğrula ve SharedMemory nesnesini bul.
                 // 2. Geçerli görevin adres alanında boş bir sanal adres aralığı bul.
                 // 3. SharedMemory'nin fiziksel sayfalarını bu sanal adres aralığına eşle (sayfa tablolarını güncelle).
                 // 4. TLB'yi flush et (sfence.vma).
                 // 5. Atanan sanal adresin başlangıcını döndür.

                 // Şimdilik yer tutucu
                 kernel_error_to_i64(SahneError::NotSupported) // Henüz desteklenmiyor
                 // Bu kısım ciddi implementasyon gerektirir (MMU/Paging, sanal bellek yönetimi).
            }
             arch::SYSCALL_SHARED_MEM_UNMAP => {
                // unmap_shared(addr: *mut u8, size: usize) -> Result<(), SahneError>
                 let addr = arg0 as *mut u8;
                 let size = arg1 as usize;
                 // TODO: Kernel-internal shared memory management logic (MMU/Paging ile ilgili)
                 // 1. Belirtilen sanal adres aralığının geçerli görevin adres alanında eşlenmiş SharedMemory olup olmadığını doğrula.
                 // 2. Sayfa tablosundaki eşlemeleri kaldır.
                 // 3. TLB'yi flush et (sfence.vma).
                 // 4. Başarı veya hata kodunu döndür.

                 // Şimdilik yer tutucu
                 kernel_error_to_i64(SahneError::NotSupported) // Henüz desteklenmiyor
                 // Bu kısım ciddi implementasyon gerektirir (MMU/Paging, sanal bellek yönetimi).
            }
            arch::SYSCALL_TASK_YIELD => {
                  yield_now() -> Result<(), SahneError>
                 // TODO: scheduler'da yield fonksiyonunu çağır.
                 sched::task_yield(); // sched::task_yield Result dönebilir
                 0i64 // Başarılı varsayalım
                 // sched::task_yield Result dönerse burası güncellenmeli.
            }

            _ => {
                // Bilinmeyen sistem çağrısı
                // printk!("Bilinmeyen Sistem Çağrısı! Numara: {}\n", syscall_num);
                // Görev sonlandırılabilir veya hata kodu döndürülebilir.
                kernel_error_to_i64(SahneError::UnknownSystemCall) // Bilinmeyen syscall hata kodu
            }
        };

        // Dönüş değerini a0 registerına yaz
        (*trap_frame).a0 = return_value as u64;

        // Hata kodu ABI'de a1'de de dönüyorsa, onu da ayarlayın.
        // Sahne64'ün map_kernel_error fonksiyonu sadece a0'daki negatif değere bakıyor,
        // bu yüzden a1'i şimdilik 0 olarak bırakabiliriz.
         (*trap_frame).a1 = 0;
    }
}

// TODO: Kernel-internal resource management logic (ResourceHandle, Kaynak Tablosu vb.)
// Bu karmaşık logic genellikle ayrı bir modülde (örn. main_kernel/resource_manager.rs) yer alır.
// Syscall handler'lar bu modülün fonksiyonlarını çağırır.

// TODO: Kernel-internal lock management logic (KernelMutex, ConditionVariable vb.)
// Bu karmaşık logic de ayrı bir modülde (örn. main_kernel/sync_manager.rs) yer alabilir.
// Syscall handler'lar bu modülün fonksiyonlarını çağırır.

// TODO: Kernel-internal time source logic (Sistem saatini takip etme)
// Bu logic de ayrı bir modülde (örn. main_kernel/kernel_time.rs) yer alabilir.
// Syscall handler'lar bu modülün fonksiyonlarını çağırır.


// TODO: mm/memory.rs'de syscall'lar için public allocate/deallocate fonksiyonları
 #[no_mangle] pub extern "C" fn sys_allocate(size: usize) -> Result<*mut u8, SahneError> { ... }
 #[no_mangle] pub extern "C" fn sys_deallocate(ptr: *mut u8, size: usize) -> Result<(), SahneError> { ... }
 #[no_mangle] pub extern "C" fn sys_create_shared(size: usize) -> Result<u64, SahneError> { ... } // Handle döner
 #[no_mangle] pub extern "C" fn sys_map_shared(handle: u64, offset: usize, size: usize) -> Result<*mut u8, SahneError> { ... }
 #[no_mangle] pub extern "C" fn sys_unmap_shared(addr: *mut u8, size: usize) -> Result<(), SahneError> { ... }


// TODO: sched.rs'de syscall'lar için public thread fonksiyonları
 #[no_mangle] pub extern "C" fn sys_create_thread(entry_point: usize, stack_size: usize, arg: u64) -> Result<u64, SahneError> { ... } // Thread ID döner
 #[no_mangle] pub extern "C" fn sys_sleep(milliseconds: u64) -> Result<(), SahneError> { ... }
 #[no_mangle] pub extern "C" fn sys_yield() -> Result<(), SahneError> { ... }


// TODO: exit.rs'de thread exit için public fonksiyon
 #[no_mangle] pub extern "C" fn sys_exit_thread(code: i32) -> ! { ... }
// Şu an Task Exit ile aynı (exit::sys_exit) kullanılabilir.