// desktop_environment/sahnedesktop/src/main.rs
// SahneBox Masaüstü Ortamı (sahnedesktop)
// Dokunmatik ekrana uyumlu GUI

#![no_std]
#![feature(alloc)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use core::fmt::Write;
use core::slice;
use core::ptr;


// SahneBox Çekirdek API'si
use crate::sahne64::{self, resource, memory, task, SahneError, Handle};

// Minimal Dosya Sistemi Kütüphanesi
use crate::filesystem::ext::ExtFilesystem;

// Minimal UI Araç Seti Kütüphanesi
use crate::minimal_gtk4::{self, Application, Widget, Label, Button, VBox}; // minimal_gtk4 modülünü kullan


// TODO: JPEG Çözücü Kütüphanesi
// no_std uyumlu bir JPEG çözücü kütüphanesi (örn. 'jpeg-decoder' crate'inin portu veya basitleştirilmiş versiyonu)
// Veya bu kütüphaneyi kendiniz yazmanız gerekir (ÇOK ZOR).
 use crate::jpeg_decoder_minimal; // Varsayımsal JPEG çözücü kütüphanesi

// Çözülmüş Resim Verisi Yapısı
struct DecodedImage {
    pixel_data: Vec<u8>, // Ham piksel verisi (örn. RGB veya RGBA baytları)
    width: u32,
    height: u32,
    // TODO: Piksel formatı (örn. RGB, RGBA)
}

// TODO: resource::write üzerine yazıcı wrapper'ı (Diğer uygulamalardan kopyalandı)
struct ConsoleWriter { handle: Handle, }
impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        resource::write(self.handle, s.as_bytes(), 0, s.as_bytes().len()).unwrap_or(0);
        Ok(())
    }
}


// Arkaplan Resmini Yükleme ve Çözme Fonksiyonu
// Dosya sisteminden JPEG dosyasını okur ve çözer.
fn load_and_decode_background(fs: &ExtFilesystem, console: &mut ConsoleWriter) -> Result<DecodedImage, SahneError> {
    let file_path = "Arkaplan.jpg"; // Arka plan dosyası yolu

    // 1. Dosyayı bul ve oku
    let root_inode = fs.root_directory()?;
    let file_inode_num = fs.lookup(root_inode.inode, file_path).unwrap_or(0);

    if file_inode_num == 0 {
        writeln!(console, "Hata: Arka plan dosyası {} bulunamadı.", file_path).unwrap();
        return Err(SahneError::ResourceNotFound);
    }

    let file_inode = fs.read_inode(file_inode_num)?;
    let file_size = file_inode.i_size as usize;

    if file_size == 0 {
         writeln!(console, "Hata: Arka plan dosyası {} boş.", file_path).unwrap();
         return Err(SahneError::InvalidOperation);
    }

    let mut jpeg_data = alloc::vec![0u8; file_size];
    let bytes_read = fs.read_file(&file_inode, &mut jpeg_data, 0)?; // Dosyanın tamamını oku

    if bytes_read != file_size {
         writeln!(console, "Hata: Arka plan dosyası tam okunamadı ({} / {}).", bytes_read, file_size).unwrap();
         return Err(SahneError::InvalidOperation);
    }


    // 2. JPEG verisini çöz (Decode)
    // Bu kısım JPEG çözücü kütüphanesini gerektirir.
    writeln!(console, "Arka plan resmi çözülüyor...").unwrap();
    // TODO: jpeg_decoder_minimal::decode(&jpeg_data) -> Result<DecodedImage, JpegError> gibi
    // Geçici olarak sabit bir DecodedImage yapısı dönelim
     printk!("WARN: JPEG çözme implemente edilmedi, varsayılan resim kullanılıyor.");
    let dummy_pixel_data: Vec<u8> = alloc::vec![0xFF; 800 * 600 * 4]; // Beyaz ARGB varsayımı
    let dummy_image = DecodedImage { pixel_data: dummy_pixel_data, width: 800, height: 600 };

    // Match gerçek çözme fonksiyonunu çağıracak
     match jpeg_decoder_minimal::decode(&jpeg_data) {
         Ok(decoded_image) => {
              writeln!(console, "Arka plan resmi başarıyla çözüldü ({}x{}).", decoded_image.width, decoded_image.height).unwrap();
              Ok(decoded_image)
         }
         Err(err) => {
              writeln!(console, "Hata: Arka plan resmi çözülemedi: {:?}", err).unwrap();
              Err(SahneError::InvalidParameter) // Veya özel bir hata
         }
     }
     writeln!(console, "Arka plan resmi çözüldü (varsayımsal 800x600).").unwrap();
     Ok(dummy_image) // Başarılı varsayalım

}

// Çözülmüş Resmi Pencere Tamponuna Çizme
// Resmi pencerenin tüm alanına yayar (basit ölçekleme veya sadece kopyalama).
fn draw_background_image(
    painter: &mut minimal_gtk4::Painter, // Pencere tamponu üzerine çizim yapan painter
    image: &DecodedImage,
) {
    // TODO: Resim formatı (image.pixel_data'nın formatı) ile painter'ın buffer formatı uyumlu mu kontrol et/dönüştür.
    // Örn: JPEG RGBA döner, framebuffer ARGB ister, bayt sıralaması farklı olabilir.

    // Basitlik için resmin pencere boyutuyla aynı olduğunu ve formatın uyduğunu varsayalım.
    if image.width == painter.width && image.height == painter.height {
        // Doğrudan kopyala
        let buffer_slice = unsafe { slice::from_raw_parts_mut(painter.buffer.as_ptr(), painter.width as usize * painter.height as usize * painter.pixel_size as usize) };
        buffer_slice.copy_from_slice(&image.pixel_data);
    } else {
        // TODO: Resmi pencere boyutuna ölçekle veya ortala ve çiz.
        // Bu, piksel interpolasyonu gerektirir (KALKOMPLEKS).
        writeln!(ConsoleWriter {handle: resource::acquire("console", resource::MODE_WRITE).unwrap()},
                 "WARN: Arka plan resmi boyutu pencereyle eşleşmiyor. Ölçekleme/ortalama implemente edilmedi.").unwrap();
         // Şimdilik sadece köşeye çizelim veya atlayalım
         // Örnek: Köşeye çizme (resim pencereden büyükse kesilir)
         for y in 0..core::cmp::min(image.height, painter.height) {
             for x in 0..core::cmp::min(image.width, painter.width) {
                 // Pikseli resimden al
                 let img_index = ((y * image.width + x) * (image.pixel_data.len() / (image.width * image.height)) as u32) as usize; // Piksel formatına göre index hesapla
                 let pixel_color = u32::from_le_bytes(image.pixel_data[img_index..img_index + painter.pixel_size as usize].try_into().unwrap()); // Varsayım: image pixel size == fb pixel size

                 painter.draw_pixel(x as i32, y as i32, pixel_color);
             }
         }
    }
}


// Uygulama Başlatma Yardımcısı
// Çalıştırılabilir dosyayı bulur ve çekirdeğe yeni thread olarak başlatması için syscall yapar.
// Kabukta tartıştığımız execute_program mantığının bir benzeri.
fn launch_application(program_path: &str, console: &mut ConsoleWriter, fs: &ExtFilesystem) -> Result<(), SahneError> {
    writeln!(console, "Uygulama Başlatılıyor: {}", program_path).unwrap();

    // TODO: Program dosyasını (ExtFilesystem kullanarak) bul ve oku.
    // TODO: Çalıştırılabilir formatı parse et (ELF, özel format). Giriş noktasını ve segmentleri bul.
    // TODO: Program için bellek ayır (memory::allocate).
    // TODO: Dosya içeriğini ayrılan belleğe kopyala.
    // TODO: Argümanları hazırla (argc, argv). Shell'deki parse_command_line gibi olabilir, ama burda argüman sabit.
    // TODO: sahne64::task::create_thread syscall'ı ile yeni thread başlat.

    writeln!(console, "WARN: Uygulama başlatma (executable loader) implemente edilmedi.").unwrap();
    // Geçici olarak hata döndürelim
    Err(SahneError::NotSupported)
}


// Masaüstü Ortamı Uygulamasının Ana Giriş Noktası
#[no_mangle]
pub extern "C" fn main(_argc: usize, _argv: *const *const u8) -> ! {
    // Konsol kaynağını edin (Hata ayıklama/loglama için)
    let console_handle = resource::acquire("console", resource::MODE_WRITE).unwrap_or_else(|_| { loop { core::hint::spin_loop(); } });
    let mut console_writer = ConsoleWriter { handle: console_handle };

    writeln!(console_writer, "SahneBox Masaüstü Ortamı (sahnedesktop) Başlıyor.").unwrap();

    // Dosya sistemini bağla (Arka plan resmini okumak için)
    let fs_instance = { // Scope fs_instance'ı sadece bu blokla sınırlamak için
        let target_device_handle = resource::acquire("emmc0", resource::MODE_READ).unwrap_or_else(|_| {
            writeln!(console_writer, "Hata: Hedef cihaz (emmc0) kaynağına erişilemedi.").unwrap();
            Handle::invalid()
        });

        if target_device_handle.is_valid() {
             match ExtFilesystem::mount(target_device_handle) {
                 Ok(fs) => Some(fs),
                 Err(err) => {
                     writeln!(console_writer, "Hata: Dosya sistemi bağlanamadı: {:?}", err).unwrap();
                     None
                 }
             }
        } else {
             None
        }
    };

    if fs_instance.is_none() {
        writeln!(console_writer, "Hata: Dosya sistemi olmadan masaüstü başlatılamaz.").unwrap();
        task::exit(-1); // Dosya sistemi yoksa çık
    }
    let fs = fs_instance.unwrap(); // fs_instance Some(fs) ise değeri al

    // UI araç setini (minimal_gtk4) başlat / Uygulama oluştur
    let mut app = match Application::new() {
        Ok(app) => app,
        Err(err) => {
            writeln!(console_writer, "Hata: UI uygulaması başlatılamadı: {:?}", err).unwrap();
            task::exit(-2);
        }
    };
    writeln!(console_writer, "UI Uygulaması başlatıldı.").unwrap();


    // Ana masaüstü penceresini oluştur (tam ekran)
    // Pencereleme sunucusu ekran boyutunu bilmeli ve minimal_gtk4'e iletmeli.
    let screen_width = 800; // Varsayımsal ekran boyutu
    let screen_height = 600; // Varsayımsal ekran boyutu
    let window_title = "SahneBox Desktop";

    // minimal_gtk4::Application::create_main_window çağrılır.
    // Bu fonksiyon içinde, pencereleme sunucusunda pencere oluşturulur ve pencere buffer'ına erişim alınır.
    // Arka plan çizimi, widget'ların çiziminden önce doğrudan pencere buffer'ına yapılabilir.


    // Arka plan resmini yükle ve çöz
    let background_image = load_and_decode_background(&fs, &mut console_writer).ok(); // Hata olursa None olur

    // Widget ağacını oluştur (masaüstü içeriği)
    let mut root_vbox = VBox::new(10); // Dikey kutucuk, 10 piksel boşluk

    // Uygulama başlatma butonları (Dokunmatik ekrana uygun, büyük)
    let mut shell_button = Button::new("Kabuk (Shell)");
    shell_button.connect_clicked(|| {
        let console_handle = resource::acquire("console", resource::MODE_WRITE).unwrap();
        let mut writer = ConsoleWriter { handle: console_handle };
        // Shell uygulamasını başlat "/bin/sh64"
        // Launch application fonksiyonu fs instance'a ihtiyaç duyar, bu global olabilir veya singleton.
        writeln!(writer, "Shell başlatılıyor...").unwrap();
         launch_application("/bin/sh64", &mut writer, &GLOBAL_FS_INSTANCE).unwrap_or_else(|e| { writeln!(writer, "Shell başlatma hatası: {:?}", e).unwrap(); });
        resource::release(console_handle).unwrap();
    });
    root_vbox.add(shell_button);

    let mut spm_button = Button::new("Paket Yöneticisi (SPM)");
     spm_button.connect_clicked(|| {
        let console_handle = resource::acquire("console", resource::MODE_WRITE).unwrap();
        let mut writer = ConsoleWriter { handle: console_handle };
        // SPM uygulamasını başlat "/bin/spm"
        writeln!(writer, "SPM başlatılıyor...").unwrap();
         launch_application("/bin/spm", &mut writer, &GLOBAL_FS_INSTANCE).unwrap_or_else(|e| { writeln!(writer, "SPM başlatma hatası: {:?}", e).unwrap(); });
        resource::release(console_handle).unwrap();
    });
    root_vbox.add(spm_button);

    let mut exit_button = Button::new("Çıkış");
     exit_button.connect_clicked(|| {
        let console_handle = resource::acquire("console", resource::MODE_WRITE).unwrap();
        let mut writer = ConsoleWriter { handle: console_handle };
        writeln!(writer, "Masaüstünden çıkılıyor...").unwrap();
        resource::release(console_handle).unwrap();
        task::exit(0); // Masaüstü uygulamasını sonlandır (çekirdek belki konsol shell'e döner)
    });
    root_vbox.add(exit_button);


    // Ana pencereyi oluştur ve root widget'ı ata
    match app.create_main_window(screen_width, screen_height, window_title, root_vbox) {
        Ok(_) => {
             writeln!(console_writer, "Ana pencere oluşturuldu.").unwrap();
        }
        Err(err) => {
            writeln!(console_writer, "Hata: Ana pencere oluşturulamadı: {:?}", err).unwrap();
            task::exit(-3);
        }
    }


    // Arka plan resmini çiz (pencere oluşturulduktan sonra, olay döngüsü başlamadan önce ilk çizim)
    // Bu, minimal_gtk4::Application::run içinde yapılmalı veya pencerenin ilk çizimi sırasında.
    // Şu anki minimal_gtk4 yapısı, arka plan çizimini doğrudan desteklemiyor.
    // Bu ya custom bir "BackgroundWidget" gibi bir widget olmalı ya da pencere çizimi sırasında özel bir adım olmalı.
    // En basit yol: minimal_gtk4::Window struct'ına veya Painter'a background image çizme özelliği eklemek.
    // Varsayalım ki Application::run içinde ilk çizim yapılırken arka plan çizilir.
    // Bu, minimal_gtk4::Application::run'ın içinde background_image'a erişimi olması gerektiğini gösterir.
    // background_image'ı Application struct'ına ekleyebiliriz.
     app.background_image = background_image; // Application struct güncellenmeli


    // Uygulamanın ana olay döngüsünü çalıştır
    // Bu döngü girdi olaylarını alacak, widget'lara yönlendirecek ve çizimi yönetecek.
    writeln!(console_writer, "Olay döngüsü başlatılıyor...").unwrap();
    app.run(); // Bu fonksiyon geri dönmez

    // Buraya asla ulaşılmamalıdır.
    loop {}
}

// TODO: Application struct'ına background_image alanını ekleyin.
// TODO: minimal_gtk4::Application::run veya Window::draw içinde background_image varsa ilk olarak onu çizin.
// TODO: launch_application fonksiyonunu implemente edin (çekirdek syscall kullanarak).
// TODO: Gerekirse FS instance'ını launch_application içinden erişilebilecek global bir Mutex içinde saklayın.
 static GLOBAL_FS_INSTANCE: spin::Mutex<Option<ExtFilesystem>> = spin::Mutex::new(None);
// main içinde fs'yi mount edip buraya kaydedin. Launch application kullanırken kilitleyin.