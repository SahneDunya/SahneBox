#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// 'alloc' crate'ini kullanmak için (heap tahsisi)
#[macro_use]
extern crate alloc;

// printk! makrosunu içeri aktar (firmware_common'dan veya kernelin kendi tanımından)
#[macro_use]
extern crate firmware_common; // Varsayımsal, firmware_common'da printk! tanımlıysa

// Temel çekirdek modüllerini içeri aktar
use core::panic::PanicInfo; // Panic handler için
use riscv::register::sstatus::{self, SPP}; // S-mode CSRs
use riscv::asm; // wfi gibi Assembly instruction'ları için
use spin::Mutex; // GlobalAlloc için Mutex
use alloc::boxed::Box; // Heap kullanımı örneği


// Çekirdek alt sistem modüllerini içeri aktar
mod uart;       // UART sürücüsü (printk için)
mod mm;         // Bellek yönetimi (heap, paging)
mod traps;      // Kesme ve istisna işleme
mod sys;        // Sistem çağrısı işleme
mod sched;      // Görev zamanlayıcı
mod exit;       // Görev sonlandırma
mod fork;       // Görev oluşturma (eğer fork syscall modeliyse)
mod drivers;    // Donanım sürücüleri (timer, storage, display vb.)
// mod loader; // Kernel-side loader mantığı (eğer ayrı bir modüldeyse)


// Global Heap Tahsis Edici (Global Allocator)
// mm::init fonksiyonunda başlatılacak ve heap alanını gösterecek.
#[global_allocator]
static HEAP_ALLOCATOR: mm::LockedHeap = mm::LockedHeap::empty(); // mm modülünde LockedHeap tanımlı olmalı


// Kernel Ana Giriş Fonksiyonu
// boot.S'den çağrılır.
// hartid: İşlemci çekirdek ID'si (S21 tek çekirdekli, genellikle 0)
// dtb_address: Device Tree Blob'un bellekteki adresi (varsa, 0 olabilir)
#[no_mangle] // Assembly tarafından çağrılabilmesi için isim bozulmamalı
pub extern "C" fn kernel_main(hartid: usize, dtb_address: usize) -> ! {
    // --- 1. Çok Erken Başlatma (Assembly tarafından yapıldı) ---
    // Stack ayarı, BSS sıfırlama, stvec, sstatus(SIE), sie Assembly'de yapıldı.
    // Şu an S-mode'dayız ve kesmeler etkinleştirildi.

    // --- 2. Konsol Başlatma ---
    // printk! kullanabilmek için UART sürücüsü başlatılmalıdır.
    uart::init();
    printk!("\n"); // Temiz bir başlangıç
    printk!("SahneBox Kernel Başlıyor (RISC-V 64)\n");
    printk!("Hart ID: {}, DTB Adresi: {:#x}\n", hartid, dtb_address);


    // --- 3. Bellek Yönetimi Başlatma ---
    // Kernelin kendi bellek yöneticisini ve heap'i başlat.
    // Bu, diğer başlangıç adımları için dinamik bellek tahsisi sağlar.
    mm::init(); // Fiziksel bellek yöneticisi veya paging structları başlatılır
    // TODO: Heap alanını belirle ve GlobalAlloc'u başlat.
    // Linker script veya DTB'den RAM bölgesini öğren.
    // Heap alanı genellikle .bss'den sonra kalan RAM'dir.
     let heap_start = unsafe { &end as *const u8 as usize }; // Linker scriptten end sembolü varsayımı
     let heap_size = RAM_END_ADDRESS - heap_start; // Toplam RAM boyutu - kernel boyutu
     HEAP_ALLOCATOR.lock().init(heap_start, heap_size);
     printk!("Heap Başlatıldı @ {:#x}, Boyut {}\n", heap_start, heap_size); # Placeholder adresler

    // GlobalAlloc'u test et (heap çalışıyor mu?)
    let test_box = alloc::boxed::Box::new(123);
    printk!("Heap testi başarılı: {} (pointer {:#p})\n", test_box, &*test_box);
    drop(test_box); // Belleği serbest bırak


    // --- 4. Kesme ve İstisna İşleme Başlatma ---
    // Assembly stvec'i ayarladı. Burada Rust handler'ları kurulur veya yapılandırılır.
    traps::init();
    printk!("Kesme ve İstisna İşleme Başlatıldı.\n");


    // --- 5. Sistem Çağrısı İşleme Başlatma ---
    // sys modülü, syscall handler'ları için dispatcher'ı içerir.
    // Başlatma gerekirse burada yapılır (genellikle traps::init içinde handler'ı belirlemek yeterli olabilir).
    sys::init();
    printk!("Sistem Çağrısı İşleme Başlatıldı.\n");


    // --- 6. Donanım Sürücülerini Başlatma ---
    // Temel sürücüleri başlat (timer, storage vb.)
    // DTB adresi bu sürücülerin başlatılması için kullanılabilir (cihaz adreslerini öğrenmek için).
    drivers::timer::init(); // Scheduler için kritik
    printk!("Timer Sürücüsü Başlatıldı.\n");

    // Depolama sürücüleri (eMMC, SD) - İlk programı yüklemek için gerekli
    drivers::storage::emmc::init();
    drivers::storage::sd::init();
    printk!("Depolama Sürücüleri Başlatıldı.\n");

    // Diğer sürücüler (display, touchscreen, audio) - User-space tarafından kullanılacak, kernel sadece başlatır.
    drivers::display::init();
    drivers::touchscreen::init();
    drivers::audio::init();
    printk!("Diğer Sürücüler Başlatıldı.\n");


    // --- 7. Görev Zamanlayıcıyı Başlatma ---
    // Scheduler yapılarını kur (run queues, idle task vb.)
    sched::init();
    printk!("Görev Zamanlayıcı Başlatıldı.\n");


    // --- 8. İlk Kullanıcı Alanı Görevini (Init Prosesi) Oluşturma ve Yükleme ---
    // Bu, işletim sisteminin başlangıç noktasıdır. Genellikle bir "init" programı çalıştırılır.
    // Bu program kullanıcı alanı shell'i (sh64) veya masaüstü ortamını (sahnedesktop) başlatabilir.
    // En basit senaryo: Doğrudan shell'i başlat.

    // TODO: İlk programın dosya sistemindeki yolu.
    // Kurulum sihirbazı imaj kopyaladı, kernel bu FS'i bulmalı (root device: eMMC) ve okumalı.
    // Bu, in-kernel bir dosya sistemi okuyucusu gerektirir (karmaşık!).
    // Veya, en basiti: İlk program (shell) RAM'de sabit bir adreste dursun (firmware tarafından oraya kopyalanmış).
    // Veya, in-kernel çok basit bir SBXE/flat binary loader implemente et.
    // En minimalist yol (şimdilik): Kernel, `/bin/sh64` dosyasını eMMC'deki kök FS'den okuyup yüklesin.
    // Bu, kernelin `drivers::storage::emmc`'i kullanarak eMMC'deki root FS'in başlangıcını bilmesini ve
    // temel FS okuma (en azından dosya bulma ve okuma) mantığını içermesini gerektirir.
    // Alternatif ve daha minimalist: Yükleyiciden bahsederken tasarladığımız `loader/src/lib.rs` modülünün
    // *çekirdek tarafında çalışan* bir versiyonunu düşünün, veya bu mantığı doğrudan buraya yazın.

    // Varsayımsal olarak, kernel içinde implemente edilmiş bir loader fonksiyonu kullanalım.
    // fn kernel_load_program(device_handle: Handle, file_path: &str) -> Result<LoadedProgram, SahneError>;
    // Bunun için eMMC device handle'ı ve "/bin/sh64" gibi bir yol gerekir.

    // EMMC cihaz handle'ını kernel içinde almak gerekir (resource::acquire kullanılamaz, o user-space syscall).
    // Driver init sırasında device handle çekirdek içinde saklanmalı.
     let root_device_handle = drivers::storage::emmc::get_device_handle(); // Varsayımsal kernel-internal fonksiyon

    // TODO: İlk programın yolunu belirle
     let first_program_path = "/bin/sh64"; // Örnek: Doğrudan shell

    // TODO: Programı yükle ve ilk görevi oluştur.
     match kernel_load_program(root_device_handle, first_program_path) { // Varsayımsal kernel loader fonksiyonu
         Ok(loaded_program) => {
    //          // Program yüklendi. Şimdi bu program için bir görev/thread oluştur.
    //          // Argümanları hazırla (argc=0, argv=null şimdilik)
               let (argc, argv_ptr, arg_mem_block) = loader::prepare_program_args(Vec::new()).unwrap(); // User-space loader'ı kernelde kullanmak? Hayır. Kernelin kendi arg hazırlığı.
              let (argc, argv_ptr, arg_mem_info) = prepare_kernel_args(Vec::new()); // Varsayımsal kernel arg hazırlığı fn.

    //          // sched::create_new_task kernel fonksiyonu çağrılır.
    //          // Bu fonksiyon, program belleğini (loaded_program), argümanları ve entry point'i alır.
              // Yeni bir Task struct'ı oluşturur, user stack'ini ayarlar, U-mode'a geçiş için ctx'i hazırlar.
               sched::create_new_task(loaded_program, argc, argv_ptr, arg_mem_info); // Varsayımsal

         }
         Err(err) => {
              printk!("Hata: İlk program '{}' yüklenemedi: {:?}", first_program_path, err).unwrap();
              // İlk program yüklenemezse sistem boot edemez. Hata mesajı verip dur.
              loop { cpu::halt(); } // cpu modülü gerekli
         }
     }

    // Geçici placeholder: İlk görevin oluşturulduğunu varsayalım.
    printk!("İlk kullanıcı alanı görevi oluşturuldu (placeholder).\n");


    // --- 9. Scheduler'ı Çalıştır ---
    // Kernel başlatma tamamlandı. Scheduler CPU kontrolünü devralır
    // ve ilk görevi (veya idle görevi) çalıştırmaya başlar.
    // Bu fonksiyon asla geri dönmez.
    sched::run_scheduler();

    // Buraya asla ulaşılmamalıdır.
    // Eğer ulaşılırsa, bir hata var demektir.
    printk!("Hata: Scheduler döndü! Sistem durduruluyor.\n");
    loop { asm::wfi(); } // İşlemciyi uykuya al
}


// TODO: Kernel-side program argüman hazırlığı fonksiyonu (prepare_kernel_args)
// loader::prepare_program_args'ın kernel versiyonu. Kernel alloc kullanır.
 fn prepare_kernel_args(args: Vec<String>) -> (usize, *const *const u8, KernelAllocInfo) { ... }
// KernelAllocInfo, ayrılan belleği serbest bırakmak için bilgi tutar.


// TODO: Kernel-side program yükleme fonksiyonu (kernel_load_program)
// Kernel'in kendi içinde SBXE (veya daha basit) formatını okuyan loader.
// Bu, storage sürücülerini (eMMC/SD) doğrudan kullanmalı ve in-kernel bir FS okuyucuya (minimal) ihtiyaç duyabilir
// veya sadece sabit offsetten okuma yapabilir.
 fn kernel_load_program(device_handle: KernelHandle, file_path: &str) -> Result<LoadedProgram, SahneError> { ... }
// KernelHandle, user-space Handle'dan farklı olabilir.


// Kernel Panik Handler'ı
// Bir panik olduğunda burası çağrılır.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Panik mesajını konsola yazdır
    printk!("\n*** KERNEL PANIC ***\n");
    if let Some(location) = info.location() {
        printk!("Konum: {}:{}\n", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        printk!("Mesaj: {}\n", message);
    } else {
        printk!("Bilinmeyen panik sebebi.\n");
    }
    printk!("********************\n");

    // Kesmeleri devre dışı bırak (panik sırasında daha fazla kesme olmasını önlemek için)
    unsafe {
        sstatus::clear_sie(); // Supervisor Interrupt Enable
         mip::clear_stip(); // Supervisor Timer Interrupt Pending (gerekirse)
         mip::clear_seip(); // Supervisor External Interrupt Pending (gerekirse)
    }

    // Sistem durdurulur, kurtarma yok
    loop {
        asm::wfi(); // İşlemciyi uykuya al, kesmeleri kapattık, uyanmayacak
    }
}