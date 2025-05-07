// main_kernel/serial.rs
// Düşük seviye UART (Seri Port) sürücüsü
// PacketBox üzerindeki UART donanımına erişim sağlar.

use core::fmt;
use spin::Mutex; // Mutex için 'spin' crate'ini kullanacağız. Cargo.toml'a eklemeyi unutmayın.
use crate::rs_io; // rs_io.S'deki Assembly fonksiyonlarını içeri aktar

// TODO: PacketBox üzerindeki UART donanımının temel MMIO adresini ve register offsetlerini belirleyin.
// Bu değerler SiFive S21 veya PacketBox'ın kendi donanımına özgü olacaktır.
const UART_BASE_ADDRESS: usize = 0x1000_0000; // Varsayımsal UART başlangıç adresi
const UART_TX_REG: usize = UART_BASE_ADDRESS + 0x00; // Varsayımsal Transmit Data Register offset
const UART_RX_REG: usize = UART_BASE_ADDRESS + 0x00; // Varsayımsal Receive Data Register offset (Genellikle TX ile aynı offsettir, donanıma bağlı)
const UART_LSR_REG: usize = UART_BASE_ADDRESS + 0x05; // Varsayımsal Line Status Register offset
const UART_LSR_TX_EMPTY: u8 = 0x20; // LSR registerındaki varsayımsal TX Empty biti

struct Uart {
    base_address: usize,
    // Diğer gerekli alanlar eklenebilir (yapılandırma vb.)
}

impl Uart {
    // Yeni bir Uart sürücüsü oluşturur (genellikle tek bir statik örnek olacaktır)
    const fn new(base_address: usize) -> Self {
        Uart { base_address }
    }

    // UART'ı initsiyalize eder (baudrate, data bits, parity vb. ayarları)
    // TODO: Bu fonksiyonu PacketBox UART donanımına göre doldurun.
    pub fn init(&self) {
        // Örnek: Çok temel initsiyalizasyon (gerçek donanıma göre değişir)
         rs_io::mmio_write32(self.base_address + UART_SOME_CONFIG_REG, some_config_value);
         printk!("UART initsiyalize edildi.\n"); // printk'i kullanmak için bu fonksiyon initten sonra çağrılmalı
    }

    // Bir bayt (karakter) gönderir.
    // Göndermeden önce hattın boşalmasını bekler (polling).
    pub fn putc(&self, byte: u8) {
        // Hattın boşalmasını bekle
        while unsafe { (rs_io::mmio_read32(self.base_address + UART_LSR_REG) as u8 & UART_LSR_TX_EMPTY) == 0 } {
            // Bekle... veya yield edilebilir eğer scheduler varsa
        }
        // Veri kaydına yaz
        unsafe { rs_io::mmio_write32(self.base_address + UART_TX_REG, byte as u32); }
    }

    // Bir bayt (karakter) okur.
    // Veri gelene kadar bekler (polling).
    // TODO: Eğer giriş gerekiyorsa bu fonksiyonu tamamlayın.
    pub fn getc(&self) -> Option<u8> {
        // Hattın dolu olup olmadığını kontrol et (donanıma bağlı bit)
         if unsafe { (rs_io::mmio_read32(self.base_address + UART_LSR_REG) as u8 & UART_LSR_RX_DATA_READY) != 0 } {
             Some(unsafe { rs_io::mmio_read32(self.base_address + UART_RX_REG) as u8 })
         } else {
             None
        }
    }
}

// UART örneğini bir Mutex ile korunan statik değişken olarak tanımla
// Bu, birden fazla yerden güvenli erişim sağlar (örn. kesmelerden veya birden fazla görevden).
static UART: Mutex<Uart> = Mutex::new(Uart::new(UART_BASE_ADDRESS));

// UART'ı başlatmak için genel fonksiyon
pub fn init() {
    UART.lock().init();
}

// fmt::Write trait'ini Uart struct'ı için implemente et
// Bu, Rust'ın formatlama makrolarını (write!, writeln!) UART ile kullanmayı sağlar.
impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            // Satır sonu karakterlerini Windows uyumlu hale getir (CR+LF)
            if byte == b'\n' {
                self.putc(b'\r');
            }
            self.putc(byte);
        }
        Ok(())
    }
}

// Global UART örneğine erişim sağlayan ve fmt::Write trait'ini kullanan bir yazıcı nesnesi döner.
// printk ve console katmanları bu yazıcıyı kullanacaktır.
pub fn writer() -> impl fmt::Write {
    // MutexGuard fmt::Write trait'ini implemente eder
    UART.lock()
}