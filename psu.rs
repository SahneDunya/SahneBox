// drivers/psu.rs
// Güç Kaynağı Ünitesi (PSU) Sürücüsü

use spin::Mutex;
use crate::printk;
use crate::rs_io;

// TODO: PSU denetleyicisinin gerçek MMIO adresini ve register offsetlerini belirleyin (varsa).
// Belki sadece durum okuma registerları vardır.
const PSU_CONTROLLER_BASE_ADDRESS: usize = 0xCCCC_0000; // Varsayımsal
const PSU_STATUS_REG_OFFSET: usize = 0x00; // Varsayımsal Durum Registerı
const PSU_CONTROL_REG_OFFSET: usize = 0x04; // Varsayımsal Kontrol Registerı (varsa)

// TODO: Durum biti tanımları (varsa)
const PSU_STATUS_ON_BIT: u32 = 1 << 0; // Varsayımsal Güç Açık biti


struct PowerSupply {
    controller_base: usize,
    // Diğer durum bilgileri (voltaj, sıcaklık vb.) eklenebilir
}

impl PowerSupply {
    const fn new(controller_base: usize) -> Self {
        PowerSupply { controller_base }
    }

    // PSU donanımını başlatır (varsa).
    // TODO: Gerçek donanıma göre doldurun. Belki yapılandırma gerektirmez.
    pub fn init(&self) {
        printk!("PSU sürücüsü başlatıldı (Pasif).\n"); // Belki sadece durum okunabilir
    }

    // PSU'nun şu anda açık olup olmadığını kontrol eder (eğer donanım destekliyorsa).
    #[allow(dead_code)] // Kullanılmıyorsa uyarı vermemesi için
    pub fn is_on(&self) -> bool {
        // TODO: Durum registerını oku ve ilgili biti kontrol et.
        // unsafe { (rs_io::mmio_read32(self.controller_base + PSU_STATUS_REG_OFFSET) & PSU_STATUS_ON_BIT) != 0 }
        true // Her zaman açık varsayalım (buzdolabı gibi?) veya fiziksel buton devrede
    }

    // PSU'yu yazılımsal olarak kapatmaya çalışır (eğer donanım destekliyorsa).
    // Genellikle bir shutdown sistem çağrısı tarafından çağrılır.
    #[allow(dead_code)] // Kullanılmıyorsa uyarı vermemesi için
    pub fn shutdown(&self) {
        printk!("PSU kapatma komutu gönderiliyor...\n");
        // TODO: Donanıma kapatma komutu gönder (varsa)
         unsafe { rs_io::mmio_write32(self.controller_base + PSU_CONTROL_REG_OFFSET, SHUTDOWN_COMMAND); }

        // Daha sonra sistem donmalıdır veya fiziksel güç kesilmelidir.
         loop {}
    }

    // TODO: Voltaj, sıcaklık gibi durum bilgilerini okuma fonksiyonları eklenebilir (varsa).
}

// PSU sürücüsünü korumak için global Mutex
static PSU_DRIVER: Mutex<PowerSupply> = Mutex::new(PowerSupply::new(PSU_CONTROLLER_BASE_ADDRESS));

// Sürücüyü başlatmak için dışarıdan çağrılacak fonksiyon
pub fn init() {
    PSU_DRIVER.lock().init();
}

// PSU durumunu sorgulamak için fonksiyon
#[allow(dead_code)]
pub fn is_on() -> bool {
    PSU_DRIVER.lock().is_on()
}

// Yazılımsal kapatma için fonksiyon
#[allow(dead_code)]
pub fn shutdown() {
    PSU_DRIVER.lock().shutdown();
}