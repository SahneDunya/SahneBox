// drivers/audio.rs
// Minimal Ses Donanım Sürücüsü

use spin::Mutex;
use crate::printk;
use crate::rs_io;
use crate::sahne64::resource::register_resource; // Kernel resource register fonksiyonu varsayımı

// TODO: PaketBox ses donanımının gerçek MMIO adresini ve register offsetlerini belirleyin.
const AUDIO_CONTROLLER_BASE_ADDRESS: usize = 0xFFAA_0000; // Varsayımsal
// TODO: Register offsetleri (Veri FIFO, Durum, Kontrol, Format vb.)
const AUDIO_PLAYBACK_FIFO_OFFSET: usize = 0x00; // Oynatma veri yazma registerı
const AUDIO_STATUS_OFFSET: usize = 0x04; // Durum registerı
const AUDIO_CONTROL_OFFSET: usize = 0x08; // Kontrol registerı (Format, Sample Rate, Enable vb.)

// TODO: Durum ve Kontrol Registerları için bit tanımları (varsayımsal)
const AUDIO_STATUS_PLAYBACK_READY: u32 = 1 << 0; // FIFO boş ve veri kabul etmeye hazır
const AUDIO_CONTROL_ENABLE_PLAYBACK: u32 = 1 << 0; // Oynatmayı etkinleştir

// TODO: Ses formatı tanımları (örneğin 16-bit Stereo 44100 Hz)
// Bu bilgiler kontrol registerına yazılmalıdır.


struct AudioDriver {
    base_address: usize,
    // TODO: Ses formatı, örnek hızı vb. bilgileri burada saklanabilir.
}

impl AudioDriver {
    const fn new(base_address: usize) -> Self {
        AudioDriver { base_address }
    }

    // Ses donanımını başlatır ve yapılandırır.
    // TODO: Gerçek donanıma göre doldurun. Ses formatını ayarlayın.
    pub fn init(&self) {
        printk!("Ses sürücüsü başlatılıyor...\n");
        // TODO: Donanımı resetle/initsiyalize et.
        // TODO: Varsayılan ses formatını ve örnek hızını ayarla (kontrol registerları).
         unsafe { rs_io::mmio_write32(self.base_address + AUDIO_CONTROL_OFFSET, SOME_FORMAT_BITS); }

        // TODO: Oynatmayı etkinleştir.
         unsafe { rs_io::mmio_write32(self.base_address + AUDIO_CONTROL_OFFSET, rs_io::mmio_read32(self.base_address + AUDIO_CONTROL_OFFSET) | AUDIO_CONTROL_ENABLE_PLAYBACK); }

        printk!("Ses sürücüsü başlatıldı.\n");
    }

    // Oynatma için ses verisi gönderir.
    // Buffer, yapılandırılmış ses formatında (örn. 16-bit stereo) ham örnekleri içerir.
    // Pollemeye dayalı (polling) implementasyon (FIFO boşalmasını bekle).
    pub fn play_samples(&self, buffer: &[u8]) -> Result<(), SahneError> {
        // printk!("Ses sürücüsü: {} bayt oynatılıyor...\n", buffer.len());
        // TODO: Buffer'daki baytları ses donanımının FIFO registerına yaz.
        // FIFO hazır olana kadar bekle (polling).
        // Genellikle 16-bit veya 32-bit kelimeler halinde yazılır.

        let mut bytes_sent = 0;
        while bytes_sent < buffer.len() {
            // FIFO'nun yazmaya hazır olup olmadığını kontrol et (polling)
            // TODO: Durum registerındaki ilgili biti kontrol et.
            let is_ready = unsafe { (rs_io::mmio_read32(self.base_address + AUDIO_STATUS_OFFSET) & AUDIO_STATUS_PLAYBACK_READY) != 0 };

            if is_ready {
                // Veri göndermeye hazırsa, bir miktar baytı FIFO'ya yaz.
                // FIFO boyutu ve donanımın veri genişliği (8/16/32 bit) önemlidir.
                // Örnek: 16-bit (2 bayt) örnekler gönder (Little-endian varsayımı).
                if bytes_sent + 2 <= buffer.len() {
                     let sample_bytes = &buffer[bytes_sent..bytes_sent+2];
                     let sample_u16 = u16::from_le_bytes(sample_bytes.try_into().unwrap());
                     // TODO: Donanım FIFO'su 16-bit mi 32-bit mi? Register offseti.
                     unsafe { rs_io::mmio_write32(self.base_address + AUDIO_PLAYBACK_FIFO_OFFSET, sample_u16 as u32); } // 16-bit FIFO yazma varsayımı
                     bytes_sent += 2;
                } else {
                     // Buffer'da kalan bayt sayısı 16-bit örnekten azsa
                     break; // Tamamlanmamış örnekleri atla (basitlik)
                }

            } else {
                // FIFO dolu, bekle veya yield et
                 printk!("FIFO dolu, bekle...\n");
                task::yield_now().unwrap_or_else(|_| { core::hint::spin_loop(); }); // Scheduler varsa yield
            }
        }
         printk!("Ses sürücüsü: {} bayt gönderildi.\n", bytes_sent);
        Ok(())
    }

    // TODO: Kayıt (recording) için fonksiyon: read_samples(&mut buffer) -> Result<usize, SahneError>

    // TODO: Ses formatı/örnek hızı ayarlama fonksiyonları.
     pub fn set_format(&self, format: AudioFormat) -> Result<(), SahneError>
}

// Ses sürücüsünü korumak için global Mutex
static AUDIO_DRIVER: Mutex<AudioDriver> = Mutex::new(AudioDriver::new(AUDIO_CONTROLLER_BASE_ADDRESS));

// Çekirdek başlangıcında çağrılacak init fonksiyonu
pub fn init() {
    AUDIO_DRIVER.lock().init();
    // TODO: Ses çıkışı ve girdisi için resource'ları çekirdek resource yöneticisine kaydet.
     register_resource("audio_out", AudioOutputResourceHandler); // Varsayımsal handler structları
     register_resource("audio_in", AudioInputResourceHandler);
    // Bu handler structları, resource read/write/control çağrılarını AudioDriver'a yönlendirmelidir.
    printk!("Ses kaynakları (audio_out, audio_in) kaydedildi (varsayımsal).\n");
}

// TODO: Resource handler structları (kaydedilmiş resource çağrılarını sürücüye yönlendiren)
 struct AudioOutputResourceHandler;
 impl ResourceHandler for AudioOutputResourceHandler {
     fn read(&self, ...) -> Result<usize, SahneError> { Err(SahneError::InvalidOperation) } // Çıkışa okunmaz
     fn write(&self, buffer: &[u8], offset: usize) -> Result<usize, SahneError> {
          // Offset genellikle oynatmada kullanılmaz, 0 varsayalım.
          AUDIO_DRIVER.lock().play_samples(buffer).map(|_| buffer.len()) // Playback başarılıysa yazılan byte sayısını dön
     }
//     // TODO: control fonksiyonu format/sample rate ayarlamak için kullanılabilir.
 }
 struct AudioInputResourceHandler;
 impl ResourceHandler for AudioInputResourceHandler {
     fn read(&self, buffer: &mut [u8], offset: usize) -> Result<usize, SahneError> { /* TODO: Sürücüden oku */ Ok(0) }
     fn write(&self, ...) -> Result<usize, SahneError> { Err(SahneError::InvalidOperation) } // Girişe yazılmaz
//     // TODO: control fonksiyonu format/sample rate ayarlamak için kullanılabilir.
 }