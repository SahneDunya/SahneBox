// main_kernel/traps.rs
// RISC-V Trap (İstisna ve Kesme) İşleyicisi

use core::arch::global_asm; // Assembly kodu gömmek için (isteğe bağlı, ayrı .S dosyası daha temiz)
use crate::printk; // printk! makrosunu içeri aktar
use crate::sys;    // Sistem çağrısı işleyicisini içeri aktar
use crate::sched;  // Zamanlayıcıyı içeri aktar (eğer timer kesmesi kullanılıyorsa)
use crate::asm::read_csr; // CSR okuma fonksiyonunu içeri aktar

// TODO: Trap Entry Assembly Kodu
// Bu Assembly kodu, trap'e girildiğinde CPU registerlarını kaydeder,
// Rust'taki handle_trap fonksiyonunu çağırır ve döndüğünde registerları geri yükleyip mret ile döner.
// Bu kodu ayrı bir .S dosyasına koymak (örn. main_kernel/trap_entry.S) ve Makefile/build.rs ile derlemek daha temizdir.
// Ancak burada konsepti göstermek için global_asm kullanılabilir.
// global_asm!(r#"
//     .section .text.entry
//     .global trap_entry
// trap_entry:
//     # 1. CPU registerlarını kaydet
//     # Task'ın stack'ine veya özel bir trap frame alanına kaydedilir.
//     # Bu kısım çok karmaşıktır ve TaskContext/TrapFrame yapınızla uyumlu olmalıdır.
//     # Örnek: (TaskContext'i stack'e kaydettiğinizi varsayalım)
//     addi sp, sp, -LEN_OF_TASK_CONTEXT # Stack'te yer aç
//     sd ra, 0(sp)
//     sd t0, 8(sp)
//     # ... diğer registerları kaydet ...
//     sd sp, OFFSET_OF_SP_IN_TASK_CONTEXT(sp) # Stack pointer'ı da kaydet!
//     sd mepc, OFFSET_OF_MEPC_IN_TASK_CONTEXT(sp)
//     sd mstatus, OFFSET_OF_MSTATUS_IN_TASK_CONTEXT(sp)
//     # ... diğer gerekli CSR'ları kaydet ...

//     # 2. handle_trap fonksiyonunu çağır
//     # Argüman olarak kaydedilmiş bağlamın (TaskContext/TrapFrame) adresini geçir.
//     mv a0, sp # Kaydedilmiş bağlamın adresini a0'a koy (ilk argüman)
//     call handle_trap # Rust fonksiyonunu çağır

//     # 3. Trap'ten dönmeden önce registerları geri yükle
//     # handle_trap fonksiyonu bağlamı güncellediyse (örn. mepc, a0), buradan geri yüklenir.
//     # Kaydedilmiş sp'yi geri yüklerken dikkatli olun, mevcut stack sp'yi kullanın.
//     ld mepc, OFFSET_OF_MEPC_IN_TASK_CONTEXT(sp) # Güncellenmiş mepc'yi yükle
//     ld mstatus, OFFSET_OF_MSTATUS_IN_TASK_CONTEXT(sp) # Güncellenmiş mstatus'u yükle
//     # ... diğer registerları yükle ...
//     ld ra, 0(sp)
//     ld t0, 8(sp)
//     # ... diğer registerları yükle ...

//     # 4. Stack'i temizle
//     addi sp, sp, LEN_OF_TASK_CONTEXT # Stack'te açılan yeri geri al

//     # 5. Trap'ten dön (mret)
//     # mret, mepc'ye atlar, mstatus'u günceller (örn. MPP -> SPP, MPIE -> SPIE).
//     mret
// "#); // global_asm sonu

// Yukarıdaki Assembly kodunda kullanılacak TaskContext/TrapFrame yapısının boyutu
// TODO: TaskContext yapınızın gerçek boyutunu ve register offsetlerini buraya yazın.
 const LEN_OF_TASK_CONTEXT: isize = core::mem::size_of::<sched::TaskContext>() as isize;
 const OFFSET_OF_SP_IN_TASK_CONTEXT: isize = core::mem::offset_of!(sched::TaskContext, sp) as isize;
 const OFFSET_OF_MEPC_IN_TASK_CONTEXT: isize = core::mem::offset_of!(sched::TaskContext, mepc) as isize;
 const OFFSET_OF_MSTATUS_IN_TASK_CONTEXT: isize = core::mem::offset_of!(sched::TaskContext, mstatus) as isize;


// TODO: Trap Frame Yapısı (isteğe bağlı olarak sched::TaskContext ile aynı olabilir)
// Trap entry Assembly kodunun kaydettiği registerları yansıtan yapı.
// Genellikle sched::TaskContext ile aynı veya benzer olacaktır.
// Eğer Assembly TaskContext'i stack'e kaydediyorsa, bu yapı o stack alanını temsil eder.
#[repr(C)]
#[derive(Debug)]
pub struct TrapFrame {
     // sched::TaskContext içindeki register alanlarını buraya kopyalayın
     // Assembly kodu bu sıraya göre kaydetmeli/yüklemelidir!
    ra: usize,
    t0: usize, t1: usize, t2: usize,
    a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, a7: usize,
    t3: usize, t4: usize, t5: usize, t6: usize,
    s0: usize, s1: usize, s2: usize, s3: usize, s4: usize, s5: usize, s6: usize, s7: usize,
    s8: usize, s9: usize, s10: usize, s11: usize,
    sp: usize,
    mepc: usize,
    mstatus: usize,
    // Diğer CSR'lar veya durumlar
}


// Trap işleyici fonksiyonu. Assembly'den çağrılır.
// Kaydedilmiş TrapFrame'in mutable bir işaretçisini alır.
#[no_mangle] // Assembly'den çağrılabilmesi için isim bozulmamalı
pub extern "C" fn handle_trap(trap_frame: *mut TrapFrame) {
    // 'unsafe' çünkü raw pointer kullanılıyor.
    unsafe {
        // Trap nedenini (mcause) oku
        let mcause = read_csr(0x342); // RISC-V mcause CSR adresi 0x342

        // Trap tipini belirle: Kesme mi (mcause MSB 1) yoksa İstisna mı (mcause MSB 0)?
        let is_interrupt = (mcause >> 63) & 1 == 1; // 63. bit (MSB for signed 64-bit)
        let trap_code = mcause & (!(1usize << 63)); // Neden kodu (işaretsiz)

        // Kaydedilmiş mepc'yi al (trap'in olduğu adres)
        let mepc_val = (*trap_frame).mepc;

        if is_interrupt {
            // Kesme (Interrupt)
            match trap_code {
                7 => { // Machine Timer Interrupt (MTIMER)
                     printk!("."); // Timer kesmesinin sık çalıştığını görmek için
                    // TODO: Timer donanımını bir sonraki kesme için programla.
                    // TODO: Zamanlayıcıyı çağır (eğer preemptive scheduling kullanılıyorsa)
                     sched::schedule();
                }
                // TODO: Diğer kesmeleri (harici, yazılım vb.) burada ele alın.
                _ => {
                    printk!("Bilinmeyen Kesme! Kod: {} MEPC: {:#x}\n", trap_code, mepc_val);
                    // Bilinmeyen kesmede panik veya sistemi durdur.
                    panic!("Bilinmeyen Kesme");
                }
            }
        } else {
            // İstisna (Exception)
            match trap_code {
                8 | 9 => { // Environment Call from U-mode (8) or S-mode (9) (Sistem Çağrısı)
                     printk!("Sistem Çağrısı MEPC: {:#x}\n", mepc_val);
                    // Sistem çağrısı işleyicisini çağır
                    sys::sys_call_handler(trap_frame);

                    // Sistem çağrısı tamamlandıktan sonra, yönergeyi atlamak için mepc'yi 4 artır.
                    // Aksi halde aynı sistem çağrısı tekrar çalışır.
                    (*trap_frame).mepc = mepc_val.wrapping_add(4); // Yönerge 4 bayt (RV64)
                }
                // TODO: Diğer istisnaları (örn. Page Fault, Illegal Instruction, Bus Error) ele alın.
                // Page Fault'lar (13, 15) bellek yönetimi (mm) için kritiktir.
                // Illegal Instruction (2) veya Access Fault (1, 3, 5, 7) gibi hatalarda genellikle görev sonlandırılır veya sistem panikler.
                2 => { // Illegal Instruction
                     printk!("Illegal Yönerge! MEPC: {:#x}\n", mepc_val);
                     // Görev sonlandırılabilir veya panik edilebilir.
                     panic!("Illegal Yönerge");
                }
                _ => {
                    printk!("Bilinmeyen İstisna! Kod: {} MEPC: {:#x}\n", trap_code, mepc_val);
                    // Bilinmeyen istisnada panik veya sistemi durdur.
                    panic!("Bilinmeyen İstisna");
                }
            }
        }

        // TODO: Trap'ten dönmeden önce (Assembly'ye dönmeden önce) yapılması gerekenler.
        // Örneğin, kaydedilmiş TaskContext'teki mepc veya a0 gibi dönüş değerleri güncellendi mi kontrol et.
    }
}

// Çekirdek başlatılırken trap handler'ı ayarlanır.
// mtvec CSR'ı trap_entry Assembly kodunun adresine ayarlanır.
pub fn init() {
    // TODO: trap_entry Assembly kodunun adresini al.
    // Bu, linker tarafından sağlanan bir sembol olabilir.
    extern "C" { fn trap_entry(); } // Assembly fonksiyonunu Rust'ta tanımla
    let trap_entry_addr = trap_entry as *const () as usize;

    // mtvec CSR'ına trap entry adresini yaz.
    // mtvec'in formatı donanıma bağlı olabilir (Vector table veya doğrudan adres).
    // RISC-V dokümantasyonuna bakın. Genellikle Base adres + Mode (Direct=0, Vectored=1).
    // Doğrudan mod (Direct Mode): mtvec = TRA P_ENTRY_ADDRESS | 0
    unsafe {
        crate::asm::write_csr(0x305, trap_entry_addr); // RISC-V mtvec CSR adresi 0x305
    }

     printk!("Trap handler {:#x} adresine ayarlandı.\n", trap_entry_addr);

    // Makine kesmelerini etkinleştir (MIE biti mstatus'ta).
     printk!("Makine Kesmeleri Etkinleştiriliyor...\n");
     unsafe { crate::asm::enable_interrupts(); } // Eğer enable_interrupts varsa kullanın.
    // TODO: Spesifik kesmeleri (timer, external vb.) etkinleştirmek için mie CSR'ını ayarlayın.
     unsafe { crate::asm::write_csr(0x304, some_interrupt_mask); } // RISC-V mie CSR adresi 0x304
}