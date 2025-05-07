// drivers/emmc.rs
// eMMC (Dahili Depolama) Sürücüsü (Blok Tabanlı)

use spin::Mutex;
use crate::printk;
use crate::rs_io;
use alloc::boxed::Box; // Buffer için heap tahsisi gerekebilir
use core::slice;

// TODO: eMMC denetleyicisinin gerçek MMIO adresini ve register offsetlerini belirleyin.
const EMMC_CONTROLLER_BASE_ADDRESS: usize = 0xBBBB_0000; // Varsayımsal
const EMMC_BLOCK_SIZE: usize = 512; // eMMC blok boyutu genellikle 512 bayttır
const EMMC_CAPACITY_MB: usize = 25; // Belirtilen kapasite

// TODO: eMMC Register offsetleri (Tamamen Varsayımsal - Gerçek Donanıma Bakılmalı!)
const EMMC_CMD_REG_OFFSET: usize = 0x00; // Command Register
const EMMC_ARG_REG_OFFSET: usize = 0x04; // Argument Register
const EMMC_RSP_REG_OFFSET: usize = 0x08; // Response Register
const EMMC_STATUS_REG_OFFSET: usize = 0x0C; // Status Register
const EMMC_DATA_REG_OFFSET: usize = 0x10; // Data Register (PIO için)
const EMMC_STATUS_CMD_DONE_BIT: u32 = 1 << 0; // Command Done bit
const EMMC_STATUS_DATA_READY_BIT: u32 = 1 << 1; // Data Ready bit
const EMMC_STATUS_ERROR_BIT: u32 = 1 << 7; // Error bit


struct EmmcStorage {
    controller_base: usize,
    // Diğer durum bilgileri (kart durumu, OCR, CID/CSD vb.) eklenebilir
}

impl EmmcStorage {
    const fn new(controller_base: usize) -> Self {
        EmmcStorage { controller_base }
    }

    // eMMC donanımını başlatır ve hazırlar.
    // TODO: eMMC protokolüne göre doldurun (Reset, Identify, Set Bus Width, Set Block Size vb.)
    // Bu karmaşık bir protokoldür.
    pub fn init(&self) -> Result<(), &'static str> {
        printk!("eMMC sürücüsü başlatılıyor...\n");

        // TODO: Temel eMMC initsiyalizasyon komutlarını gönder (CMD0, CMD1, CMD2, CMD3, CMD7 vb.)
         unsafe {
        //     // Örnek: CMD0 (Go idle state) gönderme
             rs_io::mmio_write32(self.controller_base + EMMC_ARG_REG_OFFSET, 0);
             rs_io::mmio_write32(self.controller_base + EMMC_CMD_REG_OFFSET, 0); // CMD0 kodu 0
        //     // Durum registerını kontrol ederek komutun tamamlanmasını bekle
             while (rs_io::mmio_read32(self.controller_base + EMMC_STATUS_REG_OFFSET) & EMMC_STATUS_CMD_DONE_BIT) == 0 { }
        //     // Hata kontrolü yap...
         }

        printk!("eMMC sürücüsü başlatıldı (Temel).\n");
        Ok(()) // Başarılı varsayalım
    }

    // Belirtilen blok numarasından veri okur.
    // buffer, en az EMMC_BLOCK_SIZE bayt boyutunda olmalıdır.
    // TODO: eMMC protokolüne göre doldurun (CMD17 Single Block Read veya CMD18 Multiple Block Read).
    pub fn read_block(&self, block_address: u32, buffer: &mut [u8]) -> Result<(), &'static str> {
        if buffer.len() < EMMC_BLOCK_SIZE {
            return Err("Okuma buffer boyutu yetersiz");
        }
         printk!("eMMC: Blok {} okunuyor...\n", block_address);

        // TODO: eMMC okuma komutlarını gönder (CMD17 veya CMD18)
        // Argüman olarak blok adresini kullan
        // Data Register (PIO) veya DMA kullanarak veriyi oku
        // Durum registerlarını kontrol ederek işlemin tamamlanmasını bekle
        unsafe {
             // Örnek (PIO - basitleştirilmiş):
              rs_io::mmio_write32(self.controller_base + EMMC_ARG_REG_OFFSET, block_address);
              rs_io::mmio_write32(self.controller_base + EMMC_CMD_REG_OFFSET, 17); // CMD17 (Single Block Read)
              while (rs_io::mmio_read32(self.controller_base + EMMC_STATUS_REG_OFFSET) & EMMC_STATUS_DATA_READY_BIT) == 0 { }
              for i in 0..EMMC_BLOCK_SIZE / 4 { // 32-bit PIO okuma varsayımı
                  let data = rs_io::mmio_read32(self.controller_base + EMMC_DATA_REG_OFFSET);
                  buffer[i*4..(i+1)*4].copy_from_slice(&data.to_le_bytes()); // Little-endian varsayımı
              }
             // Hata kontrolü...
        }

        Ok(()) // Başarılı varsayalım
    }

    // Belirtilen blok numarasına veri yazar.
    // data, en az EMMC_BLOCK_SIZE bayt boyutunda olmalıdır.
    // TODO: eMMC protokolüne göre doldurun (CMD24 Single Block Write veya CMD25 Multiple Block Write).
    pub fn write_block(&self, block_address: u32, data: &[u8]) -> Result<(), &'static str> {
         if data.len() < EMMC_BLOCK_SIZE {
            return Err("Yazma data boyutu yetersiz");
        }
         printk!("eMMC: Blok {} yazılıyor...\n", block_address);

        // TODO: eMMC yazma komutlarını gönder (CMD24 veya CMD25)
        // Argüman olarak blok adresini kullan
        // Data Register (PIO) veya DMA kullanarak veriyi yaz
        // Durum registerlarını kontrol ederek işlemin tamamlanmasını bekle
        unsafe {
            // Örnek (PIO - basitleştirilmiş):
              rs_io::mmio_write32(self.controller_base + EMMC_ARG_REG_OFFSET, block_address);
              rs_io::mmio_write32(self.controller_base + EMMC_CMD_REG_OFFSET, 24); // CMD24 (Single Block Write)
              while (rs_io::mmio_read32(self.controller_base + EMMC_STATUS_REG_OFFSET) & SOME_TX_READY_BIT) == 0 { } // TX buffer hazır olmasını bekle
              for i in 0..EMMC_BLOCK_SIZE / 4 { // 32-bit PIO yazma varsayımı
                  let data_chunk = u32::from_le_bytes(data[i*4..(i+1)*4].try_into().unwrap()); // Little-endian varsayımı
                  rs_io::mmio_write32(self.controller_base + EMMC_DATA_REG_OFFSET, data_chunk);
              }
             // Hata kontrolü...
        }

        Ok(()) // Başarılı varsayalım
    }

    // eMMC cihazının toplam blok sayısını döndürür.
    pub fn block_count(&self) -> u32 {
        // Kapasiteden hesapla (25MB = 25 * 1024 * 1024 bytes)
        // (25 * 1024 * 1024) / 512 = 51200
        (EMMC_CAPACITY_MB * 1024 * 1024 / EMMC_BLOCK_SIZE) as u32
        // TODO: Gerçek kapasiteyi cihazın CSD registerından okuyarak bulun.
    }

    // eMMC blok boyutunu döndürür.
    pub fn block_size(&self) -> usize {
        EMMC_BLOCK_SIZE
    }
}

// eMMC sürücüsünü korumak için global Mutex
static EMMC_DRIVER: Mutex<EmmcStorage> = Mutex::new(EmmcStorage::new(EMMC_CONTROLLER_BASE_ADDRESS));

// Sürücüyü başlatmak için dışarıdan çağrılacak fonksiyon
pub fn init() -> Result<(), &'static str> {
    EMMC_DRIVER.lock().init()
}

// Blok okumak için dışarıdan çağrılacak fonksiyon
pub fn read_block(block_address: u32, buffer: &mut [u8]) -> Result<(), &'static str> {
    EMMC_DRIVER.lock().read_block(block_address, buffer)
}

// Blok yazmak için dışarıdan çağrılacak fonksiyon
pub fn write_block(block_address: u32, data: &[u8]) -> Result<(), &'static str> {
    EMMC_DRIVER.lock().write_block(block_address, data)
}

// Toplam blok sayısını döndürür
pub fn block_count() -> u32 {
    EMMC_DRIVER.lock().block_count()
}

// Blok boyutunu döndürür
pub fn block_size() -> usize {
    EMMC_DRIVER.lock().block_size()
}