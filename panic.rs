// main_kernel/panic.rs
// Rust Panik İşleyici

use core::panic::PanicInfo;
use crate::printk; // printk! makrosunu içeri aktar
use crate::asm::halt_cpu; // Sistem durdurma fonksiyonunu içeri aktar

// Rust panik durumları için özel işleyici fonksiyonu.
// 'panic_handler' özelliği bu fonksiyonu gerektirir.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Kesmeleri devre dışı bırak (panik sırasında başka kesme olmaması için)
    // Dikkat: Kesmeleri devre dışı bırakmak için Assembly fonksiyonu gerekebilir.
    // Eğer sistem zaten panik halindeyse veya çoklu işlemci yoksa bu gerekmeyebilir.
     unsafe { crate::asm::disable_interrupts(); } // Eğer disable_interrupts varsa kullanın

    // Panik bilgilerini printk! kullanarak yazdır
    printk!("\nKERNEL PANIC: ");
    if let Some(location) = info.location() {
        // Panik yeri (dosya:satır)
        printk!("Konum: {}:{}:{}\n", location.file(), location.line(), location.column());
    } else {
        printk!("Konum bilgisi yok.\n");
    }

    if let Some(message) = info.message() {
        // Panik mesajı
        printk!("Mesaj: {}\n", message);
    } else {
        printk!("Mesaj yok.\n");
    }

    // TODO: Hata ayıklama için daha fazla bilgi yazdırılabilir (örn. register durumu, call stack - bu çok zor olabilir).
     printk!("CPU Registerları: ...\n");

    // Sistem durdurulur. Bu fonksiyondan asla dönülmemesi gerekir.
    printk!("Sistem durduruluyor.\n");
    unsafe { halt_cpu(); } // asm.S dosyasındaki halt_cpu fonksiyonunu çağır

    // halt_cpu sonsuz döngü veya WFI döngüsü yapmalıdır, buraya ulaşılmamalı.
    loop {}
}