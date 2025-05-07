// main_kernel/console.rs
// Çekirdek konsol çıktıları için soyutlama katmanı.
// Başlangıçta seri portu kullanır, gelecekte grafik ekranı destekleyebilir.

use core::fmt;
use spin::Mutex; // Mutex için 'spin' crate'ini kullanacağız.
use crate::serial; // Seri port sürücüsünü içeri aktar

// Konsol çıktı cihazını temsil eden enum (Gelecekte grafik ekran eklenebilir)
enum ConsoleDevice {
    Serial,
    // Framebuffer(FramebufferWriter), // TODO: Grafik ekran sürücüsü eklendiğinde
}

struct Console {
    device: ConsoleDevice,
    // Gelecekte imleç pozisyonu, renk ayarları vb. eklenebilir
}

impl Console {
    // Yeni bir Console örneği oluşturur. Başlangıçta seri porta yönlendirilir.
    const fn new() -> Self {
        // Başlangıçta seri portu kullan
        Console {
            device: ConsoleDevice::Serial,
        }
    }

    // Konsola bir karakter yazar.
    pub fn putc(&mut self, byte: u8) {
        match self.device {
            ConsoleDevice::Serial => {
                serial::writer().putc(byte);
            }
             ConsoleDevice::Framebuffer(ref mut fb_writer) => {
                 fb_writer.putc(byte); // TODO: Framebuffer yazıcısını kullan
             }
        }
    }

    // Konsoldan bir karakter okur.
    // TODO: Eğer girdi gerekiyorsa bu fonksiyonu tamamlayın.
    #[allow(dead_code)] // Kullanılmıyorsa uyarı vermemesi için
    pub fn getc(&mut self) -> Option<u8> {
        match self.device {
            ConsoleDevice::Serial => {
                serial::writer().getc()
            }
             ConsoleDevice::Framebuffer(ref mut fb_writer) => {
            //    // Grafik ekrandan girdi (dokunmatik klavye?)
                 None // Veya girdi mekanizmasını kullan
             }
        }
    }
}

// Konsol örneğini bir Mutex ile korunan statik değişken olarak tanımla
static CONSOLE: Mutex<Console> = Mutex::new(Console::new());

// Konsolu başlatmak için genel fonksiyon
// Bu fonksiyon, donanım initsiyalizasyonları (serial::init gibi) yapıldıktan sonra çağrılmalıdır.
pub fn init() {
    // Şu anda sadece seri port initsiyalize edildiği için burada ek bir şey yapmıyoruz.
     serial::init(); // Eğer seri port initsiyalizasyonu burada yapılıyorsa
    printk!("Konsol başlatıldı (Seri port yönlendirmeli).\n"); // printk'i kullanmak için bu init fonksiyonu printk_init'ten sonra çağrılmalı
}


// fmt::Write trait'ini global Console örneği için implemente et
// Bu, printk! gibi makroların Console katmanını kullanmasını sağlar.
impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            // Satır sonu karakterlerini Windows uyumlu hale getir (CR+LF) - İsteğe bağlı, serial sürücüsü de yapabilir
            if byte == b'\n' {
                self.putc(b'\r');
            }
            self.putc(byte);
        }
        Ok(())
    }
}

// printk! gibi makrolar tarafından kullanılan konsol yazıcısı nesnesi döner.
pub fn writer() -> impl fmt::Write {
     CONSOLE.lock() // MutexGuard<Console> döner, bu da fmt::Write'ı implemente eder
}