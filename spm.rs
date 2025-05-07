// package_manager/spm/src/main.rs
// SahneBox Paket Yöneticisi (Minimal Versiyon)

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
use crate::sahne64::{self, resource, memory, task, SahneError, Handle};

// Minimal EXT2 dosya sistemi kütüphanesi (Kullanıcı alanı kütüphanesi)
// Bu kütüphane, çekirdeğin resource::read/write sistem çağrılarını kullanarak çalışacaktır.
use crate::filesystem::ext::ExtFilesystem; // ext.rs dosyasını filesystem modülü altında varsayalım


// Komut Satırı Argümanları Pars Etmek İçin Basit Yardımcı
struct Args<'a> {
    args: Vec<&'a str>,
}

impl<'a> Args<'a> {
    // Kernelden gelen argc ve argv'yi alır
    fn parse(argc: usize, argv: *const *const u8) -> Self {
        let mut args_vec = Vec::new();
        unsafe {
            for i in 0..argc {
                let c_string = *argv.add(i);
                // C string'i Rust slice'a çevir (null terminator'a kadar)
                let mut len = 0;
                while *c_string.add(len) != 0 {
                    len += 1;
                }
                let slice = slice::from_raw_parts(c_string, len);
                if let Ok(arg) = str::from_utf8(slice) {
                    args_vec.push(arg);
                } else {
                    // Geçersiz UTF8 argümanlar ignore edilir
                }
            }
        }
        Args { args: args_vec }
    }

    fn get(&self, index: usize) -> Option<&'a str> {
        self.args.get(index).copied()
    }
}


// fmt::Write traitini kullanarak resource::write üzerine yazıcı wrapper'ı
// printk! gibi formatlama makrolarını kullanıcı alanında kullanmak için (Installer'dan kopyalandı).
struct ConsoleWriter {
    handle: Handle,
}

impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // resource::write'ın offset'li versiyonu varsayılıyor (offset = 0).
        let bytes_written = resource::write(self.handle, s.as_bytes(), 0, s.as_bytes().len()) // resource::write(handle, buf, offset, len)
             .unwrap_or(0); // Hataları yut

        if bytes_written == s.as_bytes().len() {
            Ok(())
        } else {
            Err(core::fmt::Error) // Yazılan bayt sayısı eşleşmezse hata
        }
    }
}


// Basit Paket Formatı (.spk) yapısı (Disk üzerindeki formatı temsil etmez, parse edildikten sonraki hali)
struct SahneBoxPackage {
    name: String,
    version: String,
    // ... diğer metadata (örn. description)
    files: Vec<PackageFile>, // Paketin içindeki dosyaların listesi
}

struct PackageFile {
    path: String, // Paketin içinde bulunduğu yol (örn. /bin/my_program)
    offset: usize, // Dosya verisinin .spk dosyasındaki offseti
    size: usize, // Dosya verisinin boyutu
    // TODO: Checksum eklenebilir
}

// .spk Paketini okuma ve parse etme fonksiyonu
// Basitlik için, package_file_handle'ın tüm .spk dosyasını temsil ettiği varsayılır.
// Karmaşık parse işlemleri burada yapılmalıdır.
fn parse_spk_package(package_file_handle: Handle) -> Result<SahneBoxPackage, SahneError> {
    // TODO: .spk dosyasını okuyun ve parse edin.
    // Örnek:
    // 1. Dosyanın başından metadata okuyun (isim, versiyon, dosya sayısı).
    // 2. Dosya listesini okuyun (yol, offset, boyut).
    // 3. PackageFile structlarını oluşturun.
    // Bu çok basitleştirilmiş bir örnektir. Gerçek parse karmaşıktır.

    // Şimdilik yer tutucu implementasyon: Sabit bir paket döndür
    // Bu, sadece yapının nasıl olacağını gösterir. Gerçekte dosyadan okunmalıdır.
    let dummy_package = SahneBoxPackage {
        name: "my_dummy_app".to_string(),
        version: "1.0.0".to_string(),
        files: vec![
            PackageFile {
                path: "/bin/my_dummy_app".to_string(),
                offset: 1024, // Verinin başladığı varsayımsal offset
                size: 5120,  // Varsayımsal boyut (5KB)
            },
            PackageFile {
                path: "/etc/my_dummy_app.conf".to_string(),
                offset: 1024 + 5120, // Bir önceki dosyanın bittiği yerden sonra
                size: 256,
            },
        ],
    };

    Ok(dummy_package) // Başarılı varsayalım
}


// Paketi Kurma Fonksiyonu
// package: Parse edilmiş paket yapısı.
// source_fs: Paketin bulunduğu dosya sistemi (genellikle kurulum medyası).
// target_fs: Kurulumun yapılacağı dosya sistemi (genellikle eMMC).
fn install_package(
    package: &SahneBoxPackage,
    package_file_handle: Handle, // .spk dosyasının handle'ı
    target_fs: &mut ExtFilesystem,
    console: &mut ConsoleWriter,
) -> Result<(), SahneError> {
    writeln!(console, "Paket Kuruluyor: {} (v{})", package.name, package.version).unwrap();

    // Kurulum veritabanına paket adını ekle (Basit dosya)
    // TODO: "/etc/spm/installed.list" dosyasını aç/oluştur ve paket adını ekle.
    // Bu, ExtFilesystem'in yazma yeteneği gerektirir.
    // let mut installed_list_file = target_fs.open_file("/etc/spm/installed.list", resource::MODE_WRITE | resource::MODE_CREATE | resource::MODE_APPEND)?; // open_file fonksiyonu yok

    // Paketin içindeki dosyaları hedef dosya sistemine kopyala
    for file in &package.files {
        writeln!(console, "  Dosya Kopyalanıyor: {}", file.path).unwrap();

        // TODO: Hedef dizinleri oluştur (ExtFilesystem::create_directory gerektirir)
        // Örn: target_fs.create_directory_all(parent_dir_of(file.path))?;

        // Hedef dosyayı oluştur/aç (ExtFilesystem::open_file/create_file gerektirir)
        // let mut target_file_handle = target_fs.create_file(&file.path)?; // create_file fonksiyonu yok

        // Dosya verisini kaynaktan oku (.spk dosyasından)
        let mut file_buffer = alloc::vec![0u8; file.size];
        // resource::read(package_file_handle, &mut file_buffer, file.offset, file.size)?; // resource::read'in offsetli versiyonu

        // Dosya verisini hedefe yaz (ExtFilesystem::write_file gerektirir)
        // target_fs.write_file(target_file_handle, &file_buffer)?; // write_file fonksiyonu yok

        // Handle'ları serbest bırak
        // resource::release(target_file_handle)?;
    }

    writeln!(console, "{} paketi başarıyla kuruldu.", package.name).unwrap();

    Ok(())
}

// Yüklü Paketleri Listeleme Fonksiyonu
// TODO: "/etc/spm/installed.list" dosyasını oku ve içeriğini konsola yazdır.
fn list_installed_packages(target_fs: &mut ExtFilesystem, console: &mut ConsoleWriter) -> Result<(), SahneError> {
     writeln!(console, "Yüklü Paketler:").unwrap();

     // TODO: "/etc/spm/installed.list" dosyasını bul ve oku.
     // let installed_file_inode = target_fs.lookup(root_dir.inode, "/etc/spm/installed.list")?;
     // let installed_file_inode = target_fs.read_inode(installed_file_inode_num)?;
     // let mut content_buffer = alloc::vec![0u8; installed_file_inode.i_size as usize];
     // target_fs.read_file(&installed_file_inode, &mut content_buffer, 0)?;
     // let content_str = str::from_utf8(&content_buffer).unwrap_or("<geçersiz içerik>");

     // writeln!(console, "{}", content_str).unwrap();

     writeln!(console, "- ÖrnekPaket1 v1.0.0").unwrap(); // Yer tutucu
     writeln!(console, "- ÖrnekPaket2 v2.1.0").unwrap(); // Yer tutucu

     Ok(())
}


// Paket Yöneticisi Uygulamasının Ana Giriş Noktası
#[no_mangle]
pub extern "C" fn main(argc: usize, argv: *const *const u8) -> ! {

    // Komut satırı argümanlarını parse et
    let args = Args::parse(argc, argv);

    // Konsol kaynağını edin
    let console_handle = resource::acquire("console", resource::MODE_WRITE).unwrap_or_else(|_| { loop { core::hint::spin_loop(); } });
    let mut console_writer = ConsoleWriter { handle: console_handle };

    writeln!(console_writer, "SahneBox Paket Yöneticisi (SPM)").unwrap();

    // Hedef dosya sistemini (eMMC üzerindeki root FS) bağla
    // "emmc0" kaynağını root FS olarak varsayalım ve bağlayalım.
    let target_device_handle = resource::acquire("emmc0", resource::MODE_READ | resource::MODE_WRITE).unwrap_or_else(|_| {
        writeln!(console_writer, "Hata: Hedef cihaz (emmc0) kaynağına erişilemedi.").unwrap();
        task::exit(-1);
    });

    let mut target_fs = match ExtFilesystem::mount(target_device_handle) {
        Ok(fs) => fs,
        Err(err) => {
            writeln!(console_writer, "Hata: Hedef cihaz dosya sistemi bağlanamadı: {:?}", err).unwrap();
            task::exit(-2);
        }
    };
     writeln!(console_writer, "Hedef dosya sistemi bağlandı.").unwrap();


    // Komutları işle
    match args.get(1) { // İlk argüman komut olmalı (örn. "install", "list")
        Some("install") => {
            let package_name = args.get(2); // İkinci argüman paket adı olmalı

            if let Some(name) = package_name {
                 writeln!(console_writer, "Kurulum komutu: {}", name).unwrap();

                 // TODO: Paketi içeren kurulum medyasını (SD Kart) bul ve bağla
                 let installer_media_handle = resource::acquire("sdcard1", resource::MODE_READ).unwrap_or(Handle::invalid());
                 if !installer_media_handle.is_valid() {
                      writeln!(console_writer, "Hata: Kurulum medyası (sdcard1) bulunamadı.").unwrap();
                      task::exit(-3);
                 }
                 let installer_fs = match ExtFilesystem::mount(installer_media_handle) {
                     Ok(fs) => fs,
                     Err(err) => {
                         writeln!(console_writer, "Hata: Kurulum medyası dosya sistemi bağlanamadı: {:?}", err).unwrap();
                         task::exit(-4);
                     }
                 };
                 writeln!(console_writer, "Kurulum medyası bağlandı.").unwrap();


                 // TODO: Kurulum medyasında paketi (.spk dosyasını) bul (örn. /packages/my_package.spk)
                 let package_file_path = format!("/packages/{}.spk", name); // Paket yolu
                 let packages_dir_inode = installer_fs.root_directory().unwrap(); // Basitlik için root'taki /packages varsayımı
                 let package_inode_num = installer_fs.lookup(packages_dir_inode.inode, &package_file_path).unwrap_or(0); // lookup("/packages", "my_package.spk")

                 if package_inode_num == 0 {
                      writeln!(console_writer, "Hata: Paket dosyası {} kurulum medyasında bulunamadı.", package_file_path).unwrap();
                      task::exit(-5);
                 }

                 // TODO: Paket dosyasını aç (ExtFilesystem::open_file gerektirir)
                 // let package_file_handle = installer_fs.open_file(&package_file_path, resource::MODE_READ)?; // open_file yok

                 // TODO: Paket dosyasını oku ve parse et
                 // parse_spk_package(package_file_handle) fonksiyonu çağrılacak.
                 let dummy_package_handle = Handle(100); // Yer tutucu paket handle'ı
                 match parse_spk_package(dummy_package_handle) { // Gerçekte dosya handle'ı geçmeli
                      Ok(package) => {
                         // Paketi kur
                         match install_package(&package, dummy_package_handle, &mut target_fs, &mut console_writer) {
                             Ok(_) => {}, // Başarılı
                             Err(err) => {
                                 writeln!(console_writer, "Hata: Paket kurulumu başarısız: {:?}", err).unwrap();
                                  task::exit(-6);
                             }
                         }
                      }
                      Err(err) => {
                          writeln!(console_writer, "Hata: Paket dosyası parse edilemedi: {:?}", err).unwrap();
                           task::exit(-7);
                      }
                 }

            } else {
                 writeln!(console_writer, "Hata: Kurulacak paket adı belirtilmedi.").unwrap();
                 writeln!(console_writer, "Kullanım: spm install <paket_adi>").unwrap();
            }
        }
        Some("list") => {
            writeln!(console_writer, "Liste komutu.").unwrap();
            // Yüklü paketleri listele
             match list_installed_packages(&mut target_fs, &mut console_writer) {
                 Ok(_) => {}, // Başarılı
                 Err(err) => {
                     writeln!(console_writer, "Hata: Paket listesi okunamadı: {:?}", err).unwrap();
                      task::exit(-8);
                 }
             }
        }
        _ => {
            // Bilinmeyen komut veya argüman yok
            writeln!(console_writer, "Bilinmeyen komut.").unwrap();
            writeln!(console_writer, "Kullanım:").unwrap();
            writeln!(console_writer, "  spm install <paket_adi>").unwrap();
            writeln!(console_writer, "  spm list").unwrap();
        }
    }


    // Uygulama tamamlandı, çık
    writeln!(console_writer, "SPM Tamamlandı.").unwrap();
    task::exit(0); // Başarıyla çık
}