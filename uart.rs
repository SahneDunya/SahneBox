// drivers/uart.rs
// UART (Seri Port) Donanım Sürücüsü

use core::fmt;
use spin::Mutex;
use crate::rs_io; // rs_io.S'deki Assembly MMIO fonksiyonları

// TODO: PacketBox UART donanımının gerçek adresini ve register offsetlerini belirleyin.
const UART_BASE_ADDRESS: usize = 0x1000_0000; // Varsayımsal
const UART_TX_REG_OFFSET: usize = 0x00; // Transmit Data Register offset (Varsayımsal)
const UART_RX_REG_OFFSET: usize = 0x00; // Receive Data Register offset (Varsayımsal)
const UART_LSR_REG_OFFSET: usize = 0x05; // Line Status Register offset (Varsayımsal)
const UART_LSR_TX_EMPTY_BIT: u8 = 0x20; // LSR'deki TX Empty biti (Varsayımsal)
const UART_LSR_RX_DATA_READY_BIT: u8 = 0x01; // LSR'deki RX Data Ready biti (Varsayımsal)


struct Uart {
    base_address: usize,
    // Diğer yapılandırma alanları (baudrate, vb.) eklenebilir
}

impl Uart {
    const fn new(base_address: usize) -> Self {
        Uart { base_address }
    }

    // UART donanımını başlatır (baudrate, format vb. ayarları).
    // TODO: Gerçek donanıma göre doldurun.
    pub fn init(&self) {
        // Örnek: Varsayımsal bazı ayarlar
         unsafe { rs_io::mmio_write32(self.base_address + SOME_CONFIG_REG, SOME_VALUE); }
         printk!("UART donanım sürücüsü başlatıldı.\n"); // Bu çıktı için serial/printk'in çalışıyor olması gerekir.
    }

    // Bir bayt (karakter) gönderir. Göndermeden önce hattın boşalmasını bekler (polling).
    pub fn putc(&self, byte: u8) {
        // Transmit buffer boşalana kadar bekle (Line Status Register'ı kontrol et)
        while unsafe { (rs_io::mmio_read32(self.base_address + UART_LSR_REG_OFFSET) as u8 & UART_LSR_TX_EMPTY_BIT) == 0 } {
            // Bekle...
        }
        // Veri kaydına baytı yaz
        unsafe { rs_io::mmio_write32(self.base_address + UART_TX_REG_OFFSET, byte as u32); }
    }

    // Bir bayt okumaya çalışır. Veri varsa Some(bayt), yoksa None döner (polling).
    pub fn getc(&self) -> Option<u8> {
        // Receive buffer'da veri var mı kontrol et (Line Status Register'ı kontrol et)
        if unsafe { (rs_io::mmio_read32(self.base_address + UART_LSR_REG_OFFSET) as u8 & UART_LSR_RX_DATA_READY_BIT) != 0 } {
            // Veri kaydından baytı oku
            Some(unsafe { rs_io::mmio_read32(self.base_address + UART_RX_REG_OFFSET) as u8 })
        } else {
            None // Veri yok
        }
    }
}

// UART sürücüsünü korumak için global Mutex
static UART_DRIVER: Mutex<Uart> = Mutex::new(Uart::new(UART_BASE_ADDRESS));

// Sürücüyü başlatmak için dışarıdan çağrılacak fonksiyon
pub fn init() {
    UART_DRIVER.lock().init();
}

// Bir karakter yazmak için dışarıdan çağrılacak fonksiyon
pub fn putc(byte: u8) {
    UART_DRIVER.lock().putc(byte);
}

// Bir karakter okumak için dışarıdan çağrılacak fonksiyon
#[allow(dead_code)] // Eğer şimdilik input kullanılmıyorsa
pub fn getc() -> Option<u8> {
    UART_DRIVER.lock().getc()
}

// main_kernel/serial.rs veya console.rs bu putc/getc fonksiyonlarını kullanarak
// fmt::Write traitini implemente edebilir.