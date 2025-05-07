// main_kernel/printk.rs
// Çekirdek mesajlarını yazdırmak için kullanılan arayüz (printk!)

use core::fmt;
use crate::console; // Eğer console katmanı seri portu soyutluyorsa bunu kullanın
use crate::serial; // Doğrudan seri portu kullanmak daha minimalist olabilir

#[macro_export] // Makroyu dışarıdan erişilebilir yap
macro_rules! printk {
    ($($arg:tt)*) => ({
        // 'unsafe' bloğu, çekirdek bağlamında donanım erişimi veya
        // statik mutable kaynaklara erişim için gerekebilir.
        // serial::writer() bir MutexGuard döndüğü için, lock() çağrısı güvenlidir
        // ancak writeln! makrosu içerisindeki formatlama ve yazma işlemi
        // Potansiyel olarak unsafe donanım etkileşimlerini içerebilir (zaten serial.rs içinde var).
        // En güvenli yaklaşım, serial::writer() gibi fonksiyonların UnsafeCell kullanması
        // ve buradaki lock çağrısının güvenli olmasıdır. Spinlock güvenli kabul edilir.
        #[allow(unused_unsafe)]
        unsafe {
             core::fmt::write(serial::writer(), format_args!($($arg)*)).unwrap_or_else(|_| {
            //     // Hata durumunda ne yapılabilir? Belki çok temel bir hata mesajı yazılabilir.
            //     // Bu kadar düşük seviyede hata yönetimi zor olabilir.
                  panic!("printk format error"); // Panic etmek çok düşük seviyede iyi bir fikir değil
             });
            // Alternatif ve genellikle kernelde tercih edilen yol: writeln! makrosunu kullanmak
            use core::fmt::Write;
            let mut writer = serial::writer(); // serial::writer() bir MutexGuard<Uart> döner
            let _ = writer.write_fmt(format_args!($($arg)*)); // Hataları göz ardı et
        }
    });
}

// Bilgilendirme mesajları için kolaylık makrosu (isteğe bağlı)
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        $crate::printk!("INFO: "); // $crate:: prefix'i modül yolunu belirtir
        $crate::printk!($($arg)*);
    });
}

// Uyarı mesajları için kolaylık makrosu (isteğe bağlı)
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ({
        $crate::printk!("WARN: ");
        $crate::printk!($($arg)*);
    });
}

// Hata mesajları için kolaylık makrosu (isteğe bağlı)
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ({
        $crate::printk!("ERROR: ");
        $crate::printk!($($arg)*);
    });
}

// Çekirdeğin ilk başlangıç mesajları için
// printk! makrosu kullanıma hazır olduğunda çağrılabilir.
pub fn printk_init() {
     serial::init(); // UART init burada veya init/main.rs'de çağrılabilir
     printk!("SahneBox Kernel Başlıyor...\n");
}