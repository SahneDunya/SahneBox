// drivers/sd.rs
// SD ve Micro SD Kart Sürücüsü (Blok Tabanlı)

use spin::Mutex;
use crate::printk;
use crate::rs_io;
use alloc::boxed::Box; // Buffer için heap tahsisi gerekebilir
use core::slice;

// TODO: SD kart denetleyicisinin gerçek MMIO adresini ve register offsetlerini belirleyin.
const SD_CONTROLLER_BASE_ADDRESS: usize = 0xDDAA_0000; // Varsayımsal
const SD_BLOCK_SIZE: usize = 512; // SD kart blok boyutu genellikle 512 bayttır

// TODO: SD Kart Register offsetleri (Tamamen Varsayımsal - Gerçek Donanıma Bakılmalı!)
const SD_CMD_REG_OFFSET: usize = 0x00; // Command Register
const SD_ARG_REG_OFFSET: usize = 0x04; // Argument Register
const SD_RSP_REG_OFFSET: usize = 0x08; // Response Register
const SD_STATUS_REG_OFFSET: usize = 0x0C; // Status Register
const SD_DATA_REG_OFFSET: usize = 0x10; // Data Register (PIO için)
const SD_STATUS_CMD_DONE_BIT: u32 = 1 << 0; // Command Done bit
const SD_STATUS_DATA_READY_BIT: u32 = 1 << 1; // Data Ready bit
const SD_STATUS_ERROR_BIT: u32 = 1 << 7; // Error bit


struct SdCardReader {
    controller_base: usize,
    card_inserted: bool,
    card_initialized: bool,
    // Diğer durum bilgileri (kart boyutu, tip, RCA vb.) eklenebilir
}

impl SdCardReader {
    const fn new(controller_base: usize) -> Self {
        SdCardReader {
            controller_base,
            card_inserted: false, // Başlangıçta kart yok varsayalım
            card_initialized: false,
        }
    }

    // SD kart denetleyicisini ve kartı başlatır.
    // TODO: SD kart protokolüne göre doldurun (Power Up, Reset, CMD0, CMD8, ACMD41 vb.)
    // Bu karmaşık bir protokoldür. Kartın takılı olup olmadığını kontrol etmelidir.
    pub fn init(&mut self) -> Result<(), &'static str> {
        printk!("SD kart sürücüsü başlatılıyor...\n");

        // TODO: Kartın takılı olup olmadığını kontrol et (donanıma özel pin veya register)
         self.card_inserted = unsafe { (rs_io::mmio_read32(self.controller_base + CARD_DETECT_REG) & INSERTED_BIT) != 0 };
        self.card_inserted = true; // Şimdilik takılı varsayalım

        if !self.card_inserted {
            printk!("SD kart takılı değil.\n");
            self.card_initialized = false;
            return Err("SD kart takılı değil");
        }

        // TODO: Temel SD kart initsiyalizasyon komutlarını gönder (CMD0, CMD8, ACMD41, CMD2, CMD3 vb.)
        // Bu adım kart tipini (SDHC, SDXC, SDSC) ve kapasiteyi belirler.
         unsafe { ... }

        self.card_initialized = true;
        printk!("SD kart sürücüsü başlatıldı ve kart bulundu.\n");
        Ok(()) // Başarılı varsayalım
    }

    // Belirtilen blok numarasından veri okur.
    // buffer, en az SD_BLOCK_SIZE bayt boyutunda olmalıdır.
    // TODO: SD kart protokolüne göre doldurun (CMD17 Single Block Read veya CMD18 Multiple Block Read).
    pub fn read_block(&self, block_address: u32, buffer: &mut [u8]) -> Result<(), &'static str> {
         if !self.card_initialized { return Err("SD kart initsiyalize edilmedi"); }
         if buffer.len() < SD_BLOCK_SIZE { return Err("Okuma buffer boyutu yetersiz"); }

          printk!("SD: Blok {} okunuyor...\n", block_address);

        // TODO: SD okuma komutlarını gönder (CMD17 veya CMD18)
        // Argüman olarak blok adresini kullan (SDHC/SDXC için blok adresi, SDSC için bayt adresi)
        // Data Register (PIO) veya DMA kullanarak veriyi oku
        // Durum registerlarını kontrol ederek işlemin tamamlanmasını bekle
        unsafe {
             // Örnek (PIO - basitleştirilmiş):
              rs_io::mmio_write32(self.controller_base + SD_ARG_REG_OFFSET, block_address);
              rs_io::mmio_write32(self.controller_base + SD_CMD_REG_OFFSET, 17); // CMD17
             // ... Veriyi oku ve buffera kopyala ...
        }

        Ok(()) // Başarılı varsayalım
    }

    // Belirtilen blok numarasına veri yazar.
    // data, en az SD_BLOCK_SIZE bayt boyutunda olmalıdır.
    // TODO: SD kart protokolüne göre doldurun (CMD24 Single Block Write veya CMD25 Multiple Block Write).
    pub fn write_block(&self, block_address: u32, data: &[u8]) -> Result<(), &'static str> {
        if !self.card_initialized { return Err("SD kart initsiyalize edilmedi"); }
        if data.len() < SD_BLOCK_SIZE { return Err("Yazma data boyutu yetersiz"); }
        // printk!("SD: Blok {} yazılıyor...\n", block_address);

        // TODO: SD yazma komutlarını gönder (CMD24 veya CMD25)
        // Argüman olarak blok adresini kullan
        // Data Register (PIO) veya DMA kullanarak veriyi yaz
        // Durum registerlarını kontrol ederek işlemin tamamlanmasını bekle
         unsafe {
            // Örnek (PIO - basitleştirilmiş):
              rs_io::mmio_write32(self.controller_base + SD_ARG_REG_OFFSET, block_address);
              rs_io::mmio_write32(self.controller_base + SD_CMD_REG_OFFSET, 24); // CMD24
             // ... Veriyi data registerına yaz ...
        }

        Ok(()) // Başarılı varsayalım
    }

    // SD kartın toplam blok sayısını döndürür.
    // TODO: Gerçek kapasiteyi kartın CSD registerından okuyarak bulun.
    #[allow(dead_code)]
    pub fn block_count(&self) -> Option<u32> {
         if !self.card_initialized { return None; }
         // Örnek sabit değer dönelim, gerçekte CSD'den hesaplanmalı
         Some(50000000 / SD_BLOCK_SIZE as u32) // Örnek: 50MB varsayımsal kart boyutu
    }

    // SD kart blok boyutunu döndürür.
    #[allow(dead_code)]
    pub fn block_size(&self) -> usize {
        SD_BLOCK_SIZE
    }

    // Kartın takılı olup olmadığını kontrol eder.
     #[allow(dead_code)]
    pub fn is_inserted(&self) -> bool {
        // TODO: Kart takılı pini/registerını gerçek zamanlı olarak oku.
        self.card_inserted // Init sırasındaki durumu dönelim (basitlik)
    }
}

// SD sürücüsünü korumak için global Mutex
static SD_DRIVER: Mutex<SdCardReader> = Mutex::new(SdCardReader::new(SD_CONTROLLER_BASE_ADDRESS));

// Sürücüyü başlatmak için dışarıdan çağrılacak fonksiyon
pub fn init() -> Result<(), &'static str> {
    SD_DRIVER.lock().init()
}

// Blok okumak için dışarıdan çağrılacak fonksiyon
#[allow(dead_code)]
pub fn read_block(block_address: u32, buffer: &mut [u8]) -> Result<(), &'static str> {
    SD_DRIVER.lock().read_block(block_address, buffer)
}

// Blok yazmak için dışarıdan çağrılacak fonksiyon
#[allow(dead_code)]
pub fn write_block(block_address: u32, data: &[u8]) -> Result<(), &'static str> {
    SD_DRIVER.lock().write_block(block_address, data)
}

// Kart takılı mı kontrolü
#[allow(dead_code)]
pub fn is_inserted() -> bool {
    SD_DRIVER.lock().is_inserted()
}