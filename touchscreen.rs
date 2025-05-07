// drivers/touchscreen.rs
// Dokunmatik Ekran Sürücüsü

use spin::Mutex;
use crate::printk;
use crate::rs_io;

// TODO: Dokunmatik ekran denetleyicisinin gerçek MMIO adresini ve register offsetlerini belirleyin.
const TOUCHSCREEN_CONTROLLER_BASE_ADDRESS: usize = 0xFFBB_0000; // Varsayımsal

// TODO: Dokunmatik ekran register offsetleri (Tamamen Varsayımsal - Gerçek Donanıma Bakılmalı!)
const TOUCH_STATUS_REG_OFFSET: usize = 0x00; // Durum Registerı (örn. veri hazır, basıldı/bırakıldı)
const TOUCH_X_REG_OFFSET: usize = 0x04; // X Koordinatı Registerı
const TOUCH_Y_REG_OFFSET: usize = 0x08; // Y Koordinatı Registerı
const TOUCH_STATUS_DATA_READY_BIT: u32 = 1 << 0; // Varsayımsal Veri Hazır biti
const TOUCH_STATUS_PRESSED_BIT: u32 = 1 << 1; // Varsayımsal Basıldı biti


// Dokunmatik olay türü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchEventKind {
    Pressed,
    Moved, // Basit implementasyonda zor olabilir
    Released,
}

// Dokunmatik olay bilgisi
#[derive(Debug, Clone, Copy)]
pub struct TouchEvent {
    pub kind: TouchEventKind,
    pub x: u16, // Ekran çözünürlüğü 800x600 olduğu için u16 yeterli
    pub y: u16,
     pub pressure: u16, // Basınca duyarlıysa eklenebilir
}


struct Touchscreen {
    controller_base: usize,
    // Diğer durum bilgileri (kalibrasyon verileri vb.) eklenebilir
}

impl Touchscreen {
    const fn new(controller_base: usize) -> Self {
        Touchscreen { controller_base }
    }

    // Dokunmatik ekran donanımını başlatır (kalibrasyon vb. ayarları).
    // TODO: Gerçek donanıma göre doldurun.
    pub fn init(&self) {
        printk!("Dokunmatik ekran sürücüsü başlatıldı.\n");
        // TODO: Donanımı etkinleştir (varsa)
         unsafe { rs_io::mmio_write32(self.controller_base + ENABLE_REG_OFFSET, ENABLE_VALUE); }
    }

    // Yeni bir dokunmatik olay olup olmadığını kontrol eder ve varsa olayı döndürür (polling).
    pub fn poll_event(&self) -> Option<TouchEvent> {
        // TODO: Durum registerını oku. Veri hazır mı kontrol et.
        let status = unsafe { rs_io::mmio_read32(self.controller_base + TOUCH_STATUS_REG_OFFSET) };

        if (status & TOUCH_STATUS_DATA_READY_BIT) != 0 {
            // Veri hazır, koordinatları ve durumu oku
            let x = unsafe { rs_io::mmio_read32(self.controller_base + TOUCH_X_REG_OFFSET) as u16 };
            let y = unsafe { rs_io::mmio_read32(self.controller_base + TOUCH_Y_REG_OFFSET) as u16 };
            let kind = if (status & TOUCH_STATUS_PRESSED_BIT) != 0 {
                TouchEventKind::Pressed // Basıldı veya basılı tutuluyor
            } else {
                TouchEventKind::Released // Bırakıldı
            };

            // TODO: Okunan veriyi temizle veya bir sonraki olayı tetikle (donanıma bağlı)
             unsafe { rs_io::mmio_write32(self.controller_base + TOUCH_STATUS_REG_OFFSET, CLEAR_DATA_READY_BIT); }

            Some(TouchEvent { kind, x, y })

        } else {
            None // Veri yok
        }
    }

    // TODO: Daha gelişmiş fonksiyonlar: interrupt tabanlı olaylar, kalibrasyon, jest algılama (karmaşık).
}

// Dokunmatik ekran sürücüsünü korumak için global Mutex
static TOUCHSCREEN_DRIVER: Mutex<Touchscreen> = Mutex::new(Touchscreen::new(TOUCHSCREEN_CONTROLLER_BASE_ADDRESS));

// Sürücüyü başlatmak için dışarıdan çağrılacak fonksiyon
pub fn init() {
    TOUCHSCREEN_DRIVER.lock().init();
}

// Olay kontrol etmek için dışarıdan çağrılacak fonksiyon (polling)
pub fn poll_event() -> Option<TouchEvent> {
    TOUCHSCREEN_DRIVER.lock().poll_event()
}