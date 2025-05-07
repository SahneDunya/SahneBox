// shell/sh64/src/main.rs
// SahneBox Komut Satırı Kabuğu (Minimal Versiyon)

#![no_std] // Standart kütüphane yok
#![feature(alloc)] // Heap tahsisi için alloc feature'ı
#![feature(core_intrinsics)] // core::intrinsics::write_bytes gibi şeyler için

extern crate alloc; // Heap tahsisi için alloc crate'ini kullan

use alloc::boxed::Box;
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
use crate::filesystem::ext::ExtFilesystem; // ext.rs dosyasını filesystem modülü altında varsayalım


// Komut Satırı Argümanları Pars Etmek İçin Basit Yardımcı (Installer'dan kopyalandı)
struct Args<'a> {
    args: Vec<&'a str>,
}

impl<'a> Args<'a> {
    fn parse(argc: usize, argv: *const *const u8) -> Self {
        let mut args_vec = Vec::new();
        unsafe {
            for i in 0..argc {
                let c_string = *argv.add(i);
                let mut len = 0;
                // Güvenli olmayan C string uzunluk hesaplama (null terminator'a kadar)
                while !c_string.add(len).is_null() && *c_string.add(len) != 0 {
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


// fmt::Write traitini kullanarak resource::write üzerine yazıcı wrapper'ı (Installer'dan kopyalandı)
struct ConsoleWriter {
    handle: Handle,
}

impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes_written = resource::write(self.handle, s.as_bytes(), 0, s.as_bytes().len()) // resource::write(handle, buf, offset, len)
             .unwrap_or(0); // Hataları yut

        if bytes_written == s.as_bytes().len() {
            Ok(())
        } else {
            Err(core::fmt::Error)
        }
    }
}

// Resource::read kullanarak temel konsol okuyucu
struct ConsoleReader {
     handle: Handle,
     buffer: Vec<u8>, // Satır tamponu
     buffer_pos: usize, // Tampon içindeki mevcut pozisyon
}

impl ConsoleReader {
    fn new(handle: Handle, buffer_capacity: usize) -> Self {
        ConsoleReader {
            handle,
            buffer: Vec::with_capacity(buffer_capacity),
            buffer_pos: 0,
        }
    }

    // Tek bir karakter okumaya çalışır (pollemeye dayalı).
    fn read_char(&mut self) -> Option<u8> {
        let mut byte = [0u8; 1];
        // resource::read syscall'ı burada polling veya bloklama yapabilir.
        // Blocking syscall, scheduler tarafından ele alınmalıdır.
        match resource::read(self.handle, &mut byte, 0, 1) { // resource::read(handle, buf, offset, len)
            Ok(1) => Some(byte[0]),
            _ => None, // Veri yok veya hata
        }
    }

    // Bir satır okur (Enter'a kadar). Temel satır düzenleme (backspace) yapar.
    fn read_line(&mut self, console: &mut ConsoleWriter) -> Result<String, SahneError> {
        self.buffer.clear();
        self.buffer_pos = 0;

        loop {
            // Karakter gelene kadar bekle (polling veya blocking read)
            let byte = loop {
                if let Some(b) = self.read_char() {
                    break b;
                }
                // Eğer okuma blocking değilse, işlemciyi serbest bırak
                task::yield_now().unwrap_or_else(|_| { core::hint::spin_loop(); }); // Scheduler varsa yield
            };


            match byte {
                b'\n' | b'\r' => { // Enter tuşu
                    // Yeni satır yazdır ve döngüden çık
                    writeln!(console, "").unwrap();
                    let line = String::from_utf8(self.buffer.clone()).unwrap_or(String::new()); // Geçersiz UTF8'i boş string yap
                    return Ok(line);
                }
                0x7f | b'\x08' => { // Backspace (ASCII 127 veya 8)
                    if self.buffer_pos > 0 {
                        // Tampondan son karakteri sil
                        self.buffer_pos -= 1;
                        self.buffer.pop();
                        // Konsoldan silme: Geri git, boşluk yaz, geri git
                        write!(console, "\x08 \x08").unwrap();
                    }
                }
                _ => { // Diğer karakterler
                    // Ekrana yazdır
                    write!(console, "{}", byte as char).unwrap();
                    // Tampona ekle (kapasiteyi aşmamaya dikkat et)
                    if self.buffer.len() < self.buffer.capacity() {
                        self.buffer.push(byte);
                        self.buffer_pos += 1;
                    } else {
                        // printk!("WARN: Console buffer dolu.\n"); // Buffer dolduysa uyarı
                    }
                }
            }
        }
    }
}

// Komut Satırını Parse Etme Fonksiyonu (Basit)
// Komut satırını boşluklara göre ayırır. Tırnak işaretlerini veya diğer karmaşıklıkları desteklemez.
fn parse_command_line(line: &str) -> Vec<String> {
    line.split_whitespace() // Boşluklara göre ayır
        .map(|s| s.to_string()) // Her parçayı String'e dönüştür
        .collect() // Vektör olarak topla
}

// Çalıştırılabilir Dosyayı Bulma Fonksiyonu
// Dosya sisteminde (örn. /bin) komut adını arar.
// Dönüş değeri: Çalıştırılabilir dosyanın i-node numarası (varsa) veya hata.
fn find_executable(command: &str, fs: &ExtFilesystem) -> Result<Option<u32>, SahneError> {
    // TODO: Çalıştırılabilir yolları (örn. /bin) bir listede tutmak ve sırayla aramak gerekir (PATH gibi).
    let search_path = "/bin"; // Şimdilik sadece /bin'de arayalım

    // /bin dizininin i-node'unu bul
    let root_inode = fs.root_directory()?;
    let bin_dir_inode_num = fs.lookup(root_inode.inode, search_path).unwrap_or(0); // "/bin" i-node'unu ara

    if bin_dir_inode_num == 0 {
         printk!("WARN: /bin dizini bulunamadı.\n");
        return Ok(None); // /bin dizini yok
    }

    let bin_dir_inode = fs.read_inode(bin_dir_inode_num)?;

    // /bin dizini içinde komut adını ara
    let entries = fs.list_directory(&bin_dir_inode)?;
    for entry in entries {
        if entry.inode != 0 { // Geçersiz olmayan girişler için
            // Dizin girdisindeki dosya adını al
            // Entry'nin name alanı sabit boyutlu [u8; 255].
            // name_len'e göre slice alıp str'ye çevir.
            let entry_name = str::from_utf8(&entry.name[0..entry.name_len as usize]).unwrap_or("<geçersiz>");

            if entry_name == command {
                // printk!("DEBUG: Çalıştırılabilir bulundu: {} -> i-node {}\n", command, entry.inode);
                // Dosyanın gerçekten çalıştırılabilir olup olmadığını kontrol etmek gerek (i-node i_mode ve izinler)
                // Şimdilik sadece ada bakıyoruz.
                return Ok(Some(entry.inode)); // Bulundu, i-node numarasını döndür
            }
        }
    }

    Ok(None) // Bulunamadı
}


// Çalıştırılabilir Dosyayı Yükleme ve Çalıştırma Fonksiyonu
// Bu fonksiyon, dosya sisteminden program dosyasını okuyacak,
// belleğe yükleyecek ve çalıştırmak için çekirdeğe syscall yapacaktır.
// BU KISIM ÇOK KARMAŞIKTIR VE BİR ÇALIŞTIRILABİLİR YÜKLEYİCİ GEREKTİRİR!
fn execute_program(
    inode_number: u32, // Çalıştırılacak programın i-node numarası
    fs: &ExtFilesystem, // Dosya sistemi örneği
    args: Vec<String>, // Komut satırı argümanları
    console: &mut ConsoleWriter,
) -> Result<(), SahneError> {
    writeln!(console, "DEBUG: Program i-node {} çalıştırılıyor...", inode_number).unwrap();

    // TODO: 1. Program dosyasını (i-node'dan) okuyun. ExtFilesystem::read_file kullanılır.
    let program_inode = fs.read_inode(inode_number)?;
    // Programın boyutu: program_inode.i_size

    // TODO: 2. Dosya formatını parse edin (ELF, özel format vb.).
    // Başlangıç noktası adresi, bellek segmentleri (kod, veri), sembol tablosu (isteğe bağlı) çıkarılır.
    // Bu, ayrı bir modül veya kütüphane olabilir (örn. `loader`).
     let program_data = alloc::vec![0u8; program_inode.i_size as usize];
     fs.read_file(&program_inode, &mut program_data, 0)?;
     let entry_point = parse_executable_format(&program_data)?; // Örnek parse fonksiyonu

    // TODO: 3. Program için bellekte yer ayırın (code, data, bss, stack).
    // Bu, sahne64::memory::allocate kullanılır.
    // Kod ve veri bölümleri dosyadan okunup bu ayrılan belleğe kopyalanır.
     let program_memory = memory::allocate(...)?;

    // TODO: 4. Argümanları (argc, argv) hazırlayın.
    // Argüman stringleri ve işaretçiler bellekte (genellikle stack'te veya heap'te) düzenlenmelidir.
    // Bu bellek sahne64::memory::allocate kullanılır.
    // Argümanlar yeni oluşturulan thread'e syscall argümanları olarak veya stack'e yazılarak geçirilir.
     let (argc, argv_ptr, argv_memory) = prepare_program_args(&args)?; // Helper fonksiyon

    // TODO: 5. Çekirdekten yeni bir iş parçacığı (thread) oluşturmasını isteyin.
    // sahne64::task::create_thread syscall'ı kullanılır.
    // Entry point adresi, stack boyutu ve argümanlar syscall'a geçirilir.
    let dummy_entry_point: u64 = 0x12345678; // Varsayımsal giriş noktası adresi
    let dummy_stack_size: usize = 8192; // Varsayımsal stack boyutu (8KB)
    let dummy_arg: u64 = 0; // Varsayımsal argüman (veya argv_ptr olabilir)

    writeln!(console, "DEBUG: task::create_thread syscall çağrılıyor...").unwrap();
    match task::create_thread(dummy_entry_point, dummy_stack_size, dummy_arg) {
        Ok(thread_id) => {
            writeln!(console, "DEBUG: Yeni iş parçacığı başlatıldı, ID: {}", thread_id).unwrap();
            // TODO: Task tamamlanana kadar bekle (eğer shell bekleme modundaysa).
            // task::wait(thread_id)?; // Varsayımsal task::wait syscall'ı

            // TODO: Ayrılan program belleğini ve argüman belleğini serbest bırak.
             memory::release(...);
            Ok(()) // Başarılı
        }
        Err(err) => {
            writeln!(console, "Hata: İş parçacığı oluşturulamadı: {:?}", err).unwrap();
            // TODO: Ayrılan belleği temizle (eğer hata oluştuysa).
            Err(err) // Hata
        }
    }
}


// Dahili Komutları İşleme Fonksiyonu
fn handle_builtin_command(
    command: &str,
    args: &[String], // Argümanlar (komut adı dahil)
    console: &mut ConsoleWriter,
) -> Result<(), SahneError> {
    match command {
        "exit" => {
            let exit_code = if args.len() > 1 {
                args[1].parse::<i32>().unwrap_or(0) // Argüman varsa parse et, hata olursa 0
            } else {
                0 // Argüman yoksa 0
            };
            writeln!(console, "Kabukten çıkılıyor (kod: {})...", exit_code).unwrap();
            task::exit(exit_code); // task::exit syscall'ı çağırır ve geri dönmez
        }
        "echo" => {
            // Argümanları birleştir ve yazdır
            let message = args.iter().skip(1).map(|s| s.as_str()).collect::<Vec<&str>>().join(" ");
            writeln!(console, "{}", message).unwrap();
            Ok(())
        }
        "list" => { // Basit dizin listeleme (built-in)
             writeln!(console, "DEBUG: Dosya listeleniyor (Built-in)...").unwrap();
             // TODO: Şu anki dizini belirle (shell'in CWD'si - çok basitlik için her zaman root varsayılabilir)
             // TODO: Dosya sistemini kullan (ExtFilesystem) ve dizin içeriğini oku.
              let root_inode = fs.root_directory()?; // FS instance lazım
              let entries = fs.list_directory(&root_inode)?;
              for entry in entries {
                  // entry.name, entry.inode, entry.file_type gibi bilgileri yazdır.
                  writeln!(console, "- {}", str::from_utf8(&entry.name[0..entry.name_len as usize]).unwrap_or("<geçersiz>")).unwrap();
              }
             writeln!(console, "DEBUG: Built-in list implemente edilmedi.").unwrap();
             Ok(())
        }
        // TODO: Diğer dahili komutları ekle (cd, pwd, help vb.)
        _ => Err(SahneError::NotSupported), // Bilinmeyen dahili komut (bu durum find_executable'a düşmemeli)
    }
}


// Kabuk Uygulamasının Ana Giriş Noktası
#[no_mangle]
pub extern "C" fn main(argc: usize, argv: *const *const u8) -> ! {

    // Argümanları parse et (Kabuk programının kendi argümanları)
    let _initial_args = Args::parse(argc, argv); // Kabuğa başlangıçta argüman geçiliyorsa kullanılır

    // Konsol kaynağını edin (Okuma ve Yazma için)
    let console_handle = resource::acquire("console", resource::MODE_READ | resource::MODE_WRITE).unwrap_or_else(|_| { loop { core::hint::spin_loop(); } });
    let mut console_writer = ConsoleWriter { handle: console_handle };
    let mut console_reader = ConsoleReader::new(console_handle, 256); // 256 bayt buffer

    writeln!(console_writer, "SahneBox Komut Satırı Kabuğu (sh64) Başlıyor.").unwrap();

    // Dosya sistemini bağla (Çalıştırılabilirleri bulmak için)
     let target_device_handle = resource::acquire("emmc0", resource::MODE_READ).unwrap_or_else(|_| {
        writeln!(console_writer, "Hata: Hedef cihaz (emmc0) kaynağına erişilemedi.").unwrap();
       task::exit(-1); // Kabuk çıktığında sistem donar, şimdilik çıkmayalım
        Handle::invalid() // Geçersiz handle döndür
    });

    let fs_instance = if target_device_handle.is_valid() {
         match ExtFilesystem::mount(target_device_handle) {
             Ok(fs) => {
                  writeln!(console_writer, "Hedef dosya sistemi bağlandı.").unwrap();
                  Some(fs)
             }
             Err(err) => {
                 writeln!(console_writer, "Hata: Dosya sistemi bağlanamadı: {:?}", err).unwrap();
                  None // Dosya sistemi yok
             }
         }
    } else {
         None // Cihaz yok
    };


    // Ana Kabuk Döngüsü
    loop {
        // Komut istemini göster
        write!(console_writer, "# ").unwrap();

        // Kullanıcıdan bir satır komut oku
        let command_line = match console_reader.read_line(&mut console_writer) {
            Ok(line) => line,
            Err(_) => {
                 writeln!(console_writer, "Hata: Girdi okunamadı.").unwrap();
                 continue; // Döngüye devam et
            }
        };

        // Komut satırını parse et
        let args = parse_command_line(&command_line);

        if args.is_empty() {
            continue; // Boş satır
        }

        let command = &args[0];

        // Komutu çalıştır
        // Önce dahili komutları kontrol et
        match handle_builtin_command(command, &args, &mut console_writer) {
            Ok(_) => {
                // Dahili komut başarıyla işlendi
            }
            Err(SahneError::NotSupported) => {
                // Dahili komut değil, dosya sisteminde ara
                if let Some(fs) = &fs_instance {
                     match find_executable(command, fs) {
                         Ok(Some(inode_num)) => {
                              // Çalıştırılabilir bulundu, çalıştır
                              match execute_program(inode_num, fs, args, &mut console_writer) {
                                  Ok(_) => {}, // Program başarıyla çalıştı ve bitti
                                  Err(_) => {
                                      // Hata execute_program içinde yazdırıldı
                                  }
                              }
                         }
                         Ok(None) => {
                             // Dosya sisteminde bulunamadı
                             writeln!(console_writer, "sh64: komut bulunamadı: {}", command).unwrap();
                         }
                         Err(err) => {
                             // Dosya sistemi arama hatası
                             writeln!(console_writer, "sh64: dosya sistemi hatası ararken: {:?}", err).unwrap();
                         }
                     }
                } else {
                    // Dosya sistemi bağlanamadıysa harici komut çalıştıramayız
                    writeln!(console_writer, "sh64: dosya sistemi kullanılamıyor, sadece dahili komutlar.").unwrap();
                     // handle_builtin_command zaten BilinmeyenKomut hatası döndürecek
                }

            }
             Err(err) => {
                // Diğer dahili komut hataları
                 writeln!(console_writer, "sh64: komut hatası: {:?}", err).unwrap();
            }
        }

        // Komut işlendikten sonra döngü devam eder
    }

    // Bu fonksiyondan asla dönülmemesi beklenir.
    // Eğer döngüden çıkılırsa veya beklenmedik bir şey olursa, sistemi durdurmak en güvenlisidir.
     task::exit(1); // Normalde buraya gelinmez, ama gelirse çıksın
    // Veya sonsuz döngü:
     loop {}
}