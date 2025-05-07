// installation_system/installer_app/src/main.rs
// SahneBox İşletim Sistemi Manuel Kurulum Uygulaması (Güncel İmaj Kopyalama Versiyonu)

#![no_std] // Standart kütüphane yok
#![feature(alloc)] // Heap tahsisi için alloc feature'ı

extern crate alloc; // Heap tahsisi için alloc crate'ini kullan

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format; // format! makrosu için
use core::fmt::Write; // format! çıktısını yazmak için
use core::slice;
use core::str;
use core::ptr;


// SahneBox Çekirdek API'sini içeri aktar
use crate::sahne64::{self, resource, memory, task, kernel, SahneError, Handle};

// Minimal EXT2 dosya sistemi kütüphanesi (Sadece kaynak imaj dosyasını okumak için)
use crate::filesystem::ext::ExtFilesystem; // ext.rs dosyasını filesystem modülü altında varsayalım


// TODO: resource::write üzerine yazıcı wrapper'ı (Diğer uygulamalardan kopyalandı)
struct ConsoleWriter { handle: Handle, }
impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // resource::write'ın offset'li versiyonu varsayılıyor (offset = 0).
        resource::write(self.handle, s.as_bytes(), 0, s.as_bytes().len()).unwrap_or(0);
        Ok(())
    }
}

// TODO: resource::read üzerine okuyucu wrapper'ı ve temel satır düzenleme (Shell'den kopyalandı)
struct ConsoleReader {
     handle: Handle,
     buffer: Vec<u8>, // Satır tamponu
     // TODO: buffer_pos alanı kaldırıldı, buffer clear ediliyor.
}

impl ConsoleReader {
    fn new(handle: Handle, buffer_capacity: usize) -> Self {
        ConsoleReader {
            handle,
            buffer: Vec::with_capacity(buffer_capacity),
        }
    }

    // Tek bir karakter okumaya çalışır (pollemeye dayalı).
    fn read_char(&mut self) -> Option<u8> {
        let mut byte = [0u8; 1];
        match resource::read(self.handle, &mut byte, 0, 1) { // resource::read(handle, buf, offset, len)
            Ok(1) => Some(byte[0]),
            _ => None, // Veri yok veya hata
        }
    }

    // Bir satır okur (Enter'a kadar). Temel satır düzenleme (backspace) yapar.
    fn read_line(&mut self, console: &mut ConsoleWriter) -> Result<String, SahneError> {
        self.buffer.clear();

        loop {
            let byte = loop {
                if let Some(b) = self.read_char() {
                    break b;
                }
                task::yield_now().unwrap_or_else(|_| { core::hint::spin_loop(); }); // Scheduler varsa yield
            };

            match byte {
                b'\n' | b'\r' => { // Enter tuşu
                    writeln!(console, "").unwrap();
                    let line = String::from_utf8(self.buffer.clone()).unwrap_or(String::new());
                    return Ok(line);
                }
                0x7f | b'\x08' => { // Backspace (ASCII 127 veya 8)
                    if !self.buffer.is_empty() {
                        self.buffer.pop();
                        write!(console, "\x08 \x08").unwrap();
                    }
                }
                _ => { // Diğer karakterler
                    if self.buffer.len() < self.buffer.capacity() {
                         write!(console, "{}", byte as char).unwrap();
                         self.buffer.push(byte);
                    }
                }
            }
        }
    }
}

// TODO: Dokunmatik ekrandan input okuma (touchscreen::poll_event veya resource::read üzerine kurulmuş)
// Bu Shell ve Desktop Environment bölümlerinde tartışıldı.
// Installer'da sadece temel bir dokunma olayı algılaması yeterli olabilir.
fn wait_for_touch_or_enter(console: &mut ConsoleWriter, touchscreen_handle: Handle, console_reader: &mut ConsoleReader) {
     loop {
         // Dokunmatik ekrana dokunuldu mu kontrol et
         if touchscreen_handle.is_valid() {
             // TODO: touchscreen::poll_event() veya resource::read ile dokunma verisi alıp kontrol et.
             // Basitlik için resource::read(touchscreen_handle, ...) çağrısının Ok(bir_şey) dönmesi dokunma varsayılsın.
             let mut touch_data = [0u8; 8]; // Varsayımsal dokunma olayı boyutu
             match resource::read(touchscreen_handle, &mut touch_data, 0, touch_data.len()) {
                  Ok(bytes_read) if bytes_read > 0 => {
                     writeln!(console, "Dokunma algılandı.").unwrap();
                     break; // Dokunma algılandı, devam et
                 }
                 _ => {} // Veri yok veya hata
             }
         }

         // UART konsoldan input oku (Enter'ı bekle)
         match console_reader.read_line(console) {
             Ok(line) => {
                 if line.as_str() == "Y" || line.as_str() == "y" {
                      writeln!(console, "Onay alındı.").unwrap();
                      break;
                 } else {
                      writeln!(console, "Geçersiz giriş. 'Y' yazıp Enter'a basın veya ekrana dokunun.").unwrap();
                 }
             }
             Err(_) => { /* Hata, ignore et */ }
         }

         task::yield_now().unwrap_or_else(|_| { core::hint::spin_loop(); }); // Scheduler varsa yield
     }
}


// Kurulum Uygulamasının Ana Giriş Noktası
#[no_mangle] // Kernel tarafından çağrılabilmesi için isim bozulmamalı
pub extern "C" fn main(_argc: usize, _argv: *const *const u8) -> ! {
    // --- 1. Aşama: Temel İnit ve Kaynakları Edinme ---

    // Çekirdek tarafından başlatılan temel hizmetlerin (bellek tahsisi, syscall)
    // kullanılabilir olduğunu varsayıyoruz.

    // Konsol kaynağını edin (UART veya Grafik Ekran Metin Modu)
    let console_handle = resource::acquire("console", resource::MODE_WRITE | resource::MODE_READ).unwrap_or_else(|_| {loop{core::hint::spin_loop();}});
    let mut console_writer = ConsoleWriter { handle: console_handle };
    let mut console_reader = ConsoleReader::new(console_handle, 64); // Konsol girdisi için buffer

    writeln!(console_writer, "SahneBox Kurulum Sihirbazı Başlıyor (İmaj Kopyalama)...").unwrap();

    // Dokunmatik ekran kaynağını edin (Girdi için)
    let touchscreen_handle = resource::acquire("touchscreen", resource::MODE_READ).unwrap_or(Handle::invalid());


    // --- 2. Aşama: Cihazları Algılama ve Seçim (Basit Versiyon) ---

    // Kurulum medyası (Kaynak) olarak SD Kart'ı varsayalım.
    // Hedef cihaz olarak Dahili eMMC'yi varsayalım.
    let source_device_handle = resource::acquire("sdcard1", resource::MODE_READ).unwrap_or(Handle::invalid());
    let target_device_handle = resource::acquire("emmc0", resource::MODE_READ | resource::MODE_WRITE).unwrap_or(Handle::invalid());

    if !source_device_handle.is_valid() {
        writeln!(console_writer, "Hata: Kurulum medyası (sdcard1) bulunamadı!").unwrap();
        writeln!(console_writer, "Lütfen kurulum imajını içeren SD kartı taktığınızdan emin olun.").unwrap();
        task::exit(-1); // Hata durumu
    }
     writeln!(console_writer, "Kaynak Cihaz Algılandı: SD Kart (sdcard1)").unwrap();


    if !target_device_handle.is_valid() {
        writeln!(console_writer, "Hata: Hedef cihaz (emmc0) bulunamadı!").unwrap();
        writeln!(console_writer, "Lütfen dahili eMMC'nin bağlı olduğundan emin olun.").unwrap();
        task::exit(-2); // Hata durumu
    }
    writeln!(console_writer, "Hedef Cihaz Algılandı: Dahili eMMC (emmc0)").unwrap();


    // --- 3. Aşama: İmaj Dosyasını Bul ---

    // Kurulum imajını içeren SD karttaki dosya sistemini bağla
    let source_fs = match ExtFilesystem::mount(source_device_handle) {
        Ok(fs) => fs,
        Err(err) => {
            writeln!(console_writer, "Hata: Kaynak cihaz dosya sistemi bağlanamadı: {:?}", err).unwrap();
            task::exit(-3); // Hata durumu
        }
    };
    writeln!(console_writer, "Kaynak dosya sistemi bağlandı.").unwrap();

    // Kurulum imaj dosyasını bul (Örn: "/sahnebox.img")
    let image_file_path = "/sahnebox.img"; // Kurulum imaj dosyası adı
    let root_dir_inode = source_fs.root_directory().unwrap_or_else(|e| {
         writeln!(console_writer, "Hata: Kaynak FS root dizini okunamadı: {:?}", e).unwrap();
         task::exit(-4);
    }); // Root dizininin i-node'unu al
    let image_file_inode_num = source_fs.lookup(root_dir_inode.inode, image_file_path).unwrap_or(0);

    if image_file_inode_num == 0 {
        writeln!(console_writer, "Hata: Kurulum imaj dosyası '{}' kaynak medyada bulunamadı!", image_file_path).unwrap();
        task::exit(-5); // Hata durumu
    }
     writeln!(console_writer, "Kurulum imaj dosyası '{}' bulundu.", image_file_path).unwrap();

    let image_file_inode = source_fs.read_inode(image_file_inode_num).unwrap_or_else(|e| {
         writeln!(console_writer, "Hata: İmaj dosyası i-node okunamadı: {:?}", e).unwrap();
         task::exit(-6);
    });

    let image_size = image_file_inode.i_size as usize; // İmaj boyutu (sadece 32-bit boyut desteklenir)
    if image_size == 0 {
         writeln!(console_writer, "Hata: Kurulum imaj dosyası boş!").unwrap();
         task::exit(-7);
    }
    writeln!(console_writer, "İmaj Boyutu: {} bayt", image_size).unwrap();


    // TODO: Hedef cihazın kapasitesini kontrol et. İmaj boyutu hedef cihaza sığmalı.
    // Şu an kaynak resource API'sinde kapasite bilgisi yok. Kernel resource manager güncellenmeli.
     let target_capacity_blocks = resource::get_capacity(target_device_handle)?; // Varsayımsal


    // --- 4. Aşama: Biçimlendirme Uyarısı ve Onay ---

    writeln!(console_writer, "UYARI: Hedef cihaz (emmc0) üzerindeki tüm veriler silinecektir!").unwrap();
    writeln!(console_writer, "Devam etmek için konsola 'Y' yazıp Enter'a basın veya ekrana dokunun.").unwrap();

    wait_for_touch_or_enter(&mut console_writer, touchscreen_handle, &mut console_reader);


    // --- 5. Aşama: İmajı Hedef Cihaza Kopyalama ---

    writeln!(console_writer, "Kurulum imajı hedef cihaza (emmc0) kopyalanıyor...").unwrap();

    let device_block_size = 512; // Çoğu blok cihazın sektör boyutu 512 bayttır.
    let mut copy_buffer = alloc::vec![0u8; device_block_size]; // Kopyalama buffer'ı (bir blok boyutunda)
    let total_blocks_to_copy = (image_size + device_block_size - 1) / device_block_size;

    let mut bytes_copied = 0;

    while bytes_copied < image_size {
        let bytes_left = image_size - bytes_copied;
        let bytes_to_read_this_iter = core::cmp::min(bytes_left, device_block_size);

        // Kaynak imaj dosyasından bloğu oku
        let file_offset = bytes_copied;
        match source_fs.read_file(&image_file_inode, &mut copy_buffer[0..bytes_to_read_this_iter], file_offset) {
             Ok(bytes_read) if bytes_read == bytes_to_read_this_iter => {
                  // Başarıyla okundu
             }
             Ok(_) => {
                  writeln!(console_writer, "\nHata: İmaj dosyasından okuma eksik veya boş {}. offset.", file_offset).unwrap();
                  task::exit(-8);
             }
             Err(err) => {
                  writeln!(console_writer, "\nHata: İmaj dosyasından okuma hatası {}. offset: {:?}", file_offset, err).unwrap();
                  task::exit(-9);
             }
        }

        // Hedef cihaza bloğu yaz
        let device_offset = bytes_copied;
         match resource::write(target_device_handle, &copy_buffer[0..bytes_to_read_this_iter], device_offset, bytes_to_read_this_iter) { // resource::write'ın offset'li versiyonu kullanılıyor
             Ok(bytes_written) if bytes_written == bytes_to_read_this_iter => {
                  // Başarıyla yazıldı
             }
             Ok(_) => {
                  writeln!(console_writer, "\nHata: Hedef cihaza yazma eksik veya boş {}. offset.", device_offset).unwrap();
                  task::exit(-10);
             }
             Err(err) => {
                  writeln!(console_writer, "\nHata: Hedef cihaza yazma hatası {}. offset: {:?}", device_offset, err).unwrap();
                  task::exit(-11);
             }
        }

        bytes_copied += bytes_to_read_this_iter;

        // İlerleme göstergesi
        let blocks_copied = bytes_copied / device_block_size;
        if blocks_copied % 100 == 0 {
            write!(console_writer, ".").unwrap();
        }
    }

    writeln!(console_writer, "\nİmaj kopyalama tamamlandı ({} bayt).", bytes_copied).unwrap();


    // --- 6. Aşama: Boot Konfigürasyonu (İmaj İçine Dahil Edilmiş Varsayalım) ---

    // İmaj kopyalama stratejisinde, boot konfigürasyonu genellikle zaten imaj dosyasının bir parçasıdır.
    // Ayrı bir adım olarak bir boot config dosyası yazmaya gerek kalmaz.
    writeln!(console_writer, "Boot konfigürasyonu (imaj içinde) tamamlandı.").unwrap();


    // --- 7. Aşama: Kurulum Tamamlandı ve Yeniden Başlatma ---

    writeln!(console_writer, "SahneBox kurulumu başarıyla tamamlandı!").unwrap();
    writeln!(console_writer, "Lütfen cihazı yeniden başlatın (Reset veya Güç Kesme).").unwrap();

    // TODO: İsteğe bağlı: Yeniden başlatma syscall'ı varsa kullan.
     kernel::reboot().unwrap_or_else(|e| { writeln!(console_writer, "Yeniden başlatma syscall hatası: {:?}", e).unwrap(); });
    // Reboot syscall yoksa veya başarısız olursa, kullanıcı gücü kesmeli.


    // Kurulum uygulaması burada biter. task::exit ile çıkar.
    // Eğer reboot syscall yoksa veya başarısız olursa, burada sonsuz döngüye girer.
    loop { core::hint::spin_loop(); } // Kurulum bitti, bekle

    // Kaynak Handle'ları (console, touchscreen, source_device, target_device)
    // Görev sonlandığında kernel tarafından otomatik serbest bırakılır varsayılır.
}