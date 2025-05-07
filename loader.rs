// loader/src/lib.rs
// SahneBox Yürütülebilir Dosya Yükleyicisi (.sbxe formatı)

#![no_std]
#![feature(alloc)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::slice;
use core::ptr;
use core::mem;
use core::any::Any; // Argüman belleğini Box içinde tutmak için

// SahneBox Çekirdek API'si
use crate::sahne64::{self, memory, task, SahneError};

// Minimal Dosya Sistemi Kütüphanesi
use crate::filesystem::ext::ExtFilesystem; // ext.rs dosyasını kullanacak


// SBXE Yürütülebilir Dosya Formatı Yapıları (On-Disk Format)
#[repr(C, packed)] // C uyumluluğu ve sıkı paketleme
struct SbxeFileHeader {
    magic: u32,
    architecture: u16,
    header_size: u16,
    entry_point_offset: u32, // Relative to program load address
    num_sections: u32,
    section_header_offset: u32, // Relative to file start
}

#[repr(C, packed)]
struct SbxeSectionHeader {
    type_: u32, // Bölüm türü (type olarak "_" ekledik, 'type' keyword olduğu için)
    flags: u32,
    offset_in_file: u32,
    size_in_file: u32,
    size_in_memory: u32,
    load_address_offset: u32, // Relative to program load address
}

// SBXE Sihirli Sayısı ("SBXE" ASCII)
const SBXE_MAGIC: u32 = 0x45584253;

// Bölüm Türleri
const SBXE_SECTION_TYPE_TEXT: u32 = 1; // Code
const SBXE_SECTION_TYPE_DATA: u32 = 2; // Initialized Data
const SBXE_SECTION_TYPE_BSS: u32 = 3; // Uninitialized Data

// Yüklenmiş Program Bilgisi
// Loader tarafından program belleğe yüklendikten sonra döndürülür.
pub struct LoadedProgram {
    pub entry_point: usize, // Programın başlayacağı adres
    // TODO: Ayrılan bellek bloklarına işaretçiler ve boyutlar (program sonlandığında serbest bırakmak için)
    // Şu an sadece bir ana bellek bloğu varsayalım.
    program_memory_block: Box<dyn Any>, // Ayrılan belleği tutan Box (serbest bırakmak için)
    program_memory_ptr: *mut u8,
    program_memory_size: usize,
    // TODO: Argüman belleği (eğer program argüman alıyorsa)
    arg_memory_block: Option<Box<dyn Any>>,
}

impl LoadedProgram {
     // Program sonlandığında belleği serbest bırakmak için çağrılmalı.
     // Kernel, task sonlandığında bu struct'ı alıp drop etmeli mi?
     // Veya task exit syscall'ı belleği serbest bırakma bilgisi içermeli mi?
     // En basiti: Loader sadece allocate etsin, serbest bırakma sorumluluğu kernel veya task bitiş handlerında olsun.
     // Eğer Box kullanılıyorsa, Box drop edildiğinde bellek serbest bırakılır (GlobalAlloc sayesinde).
     // Ama Box'ı kernelin drop etmesi için Box'ı kernel struct'larına eklemek lazım.
     // Veya loader, serbest bırakılacak ptr ve size bilgisini dönmeli.
}


/// Belirtilen i-node'a sahip SBXE yürütülebilir dosyasını okur, parse eder ve belleğe yükler.
/// Başarılı olursa, programın giriş noktası adresini ve bellek bilgilerini döndürür.
pub fn load_executable(fs: &ExtFilesystem, inode_number: u32) -> Result<LoadedProgram, SahneError> {
    // 1. Yürütülebilir dosyayı oku
    let program_inode = fs.read_inode(inode_number)?;
    let file_size = program_inode.i_size as usize;

    if file_size < mem::size_of::<SbxeFileHeader>() {
        return Err(SahneError::InvalidParameter); // Dosya çok kısa
    }

    let mut program_data = alloc::vec![0u8; file_size];
    fs.read_file(&program_inode, &mut program_data, 0)?; // Dosyanın tamamını oku

    // 2. Dosya Başlığını Parse Et
    let file_header: &SbxeFileHeader = unsafe {
        let ptr = program_data.as_ptr() as *const SbxeFileHeader;
        &*ptr // Dereference
        // read_unaligned() gerekebilir eğer packed struct kullanıyorsak
         ptr.read_unaligned()
    };

    if file_header.magic != SBXE_MAGIC {
        // printk!("Hata: Geçersiz SBXE sihirli sayısı: {:#x}\n", file_header.magic);
        return Err(SahneError::InvalidParameter); // Geçersiz format
    }
    if file_header.architecture != 1 { // RISC-V 64 (varsayım)
         printk!("Hata: Desteklenmeyen mimari: {}\n", file_header.architecture);
        return Err(SahneError::NotSupported); // Yanlış mimari
    }
    if (file_header.section_header_offset as usize) < file_header.header_size as usize ||
       (file_header.section_header_offset as usize) + (file_header.num_sections as usize * mem::size_of::<SbxeSectionHeader>()) > file_size
    {
          printk!("Hata: Bölüm başlıkları dosya sınırları dışında.\n");
         return Err(SahneError::InvalidParameter); // Bölüm başlıkları geçersiz konumda
    }


    // 3. Toplam Bellek Boyutunu Hesapla ve Ayır
    let section_headers_ptr = unsafe { program_data.as_ptr().add(file_header.section_header_offset as usize) as *const SbxeSectionHeader };
    let section_headers_slice = unsafe { slice::from_raw_parts(section_headers_ptr, file_header.num_sections as usize) };

    let mut total_memory_size: usize = 0;
    let mut max_load_address = 0;

    for section_header in section_headers_slice {
        // Bölümlerin en yüksek yükleneceği adresi bul (toplam bellek bloğunun boyutu için)
        let section_end_offset = section_header.load_address_offset + section_header.size_in_memory;
        if section_end_offset > max_load_address {
            max_load_address = section_end_offset;
        }

        // TODO: Bölüm verisinin dosyada sınırları içinde olup olmadığını kontrol et.
         if section_header.offset_in_file + section_header.size_in_file > file_size { ... }

        // TODO: Bölüm bayraklarını kontrol et (okunabilir, yazılabilir, çalıştırılabilir).
    }

    total_memory_size = max_load_address as usize;
    if total_memory_size == 0 {
         // printk!("Hata: Program bölümleri tanımlı değil veya boyut 0.\n");
         return Err(SahneError::InvalidParameter); // Programın bellekte boyutu 0
    }

    // Program için bellekte tek bir bitişik blok ayır
    // Bu blok, tüm bölümleri (text, data, bss) içerecektir.
    // Align gereksinimleri olabilir, şimdilik 8 bayt hizalama varsayalım.
    let program_memory = memory::allocate(total_memory_size)?; // sahne64::memory::allocate kullan

    if program_memory.is_null() {
          printk!("Hata: Program belleği tahsis edilemedi ({} bayt).\n", total_memory_size);
         return Err(SahneError::OutOfMemory);
    }
    let program_memory_block = unsafe { Box::from_raw(program_memory as *mut u8) }; // Belleği Box'a sarmala


    // 4. Bölümleri Belleğe Yükle ve BSS'i Sıfırla
    let program_base_address = program_memory as usize;

    for section_header in section_headers_slice {
        let load_address = program_base_address + section_header.load_address_offset as usize;

        match section_header.type_ {
            SBXE_SECTION_TYPE_TEXT | SBXE_SECTION_TYPE_DATA => {
                // Dosyadan belleğe kopyala
                if section_header.size_in_file > 0 {
                    let file_data_ptr = unsafe { program_data.as_ptr().add(section_header.offset_in_file as usize) };
                    unsafe {
                        ptr::copy_nonoverlapping(file_data_ptr, load_address as *mut u8, section_header.size_in_file as usize);
                    }
                }
            }
            SBXE_SECTION_TYPE_BSS => {
                // Bellek alanını sıfırla (0'larla doldur)
                if section_header.size_in_memory > 0 {
                    unsafe {
                         // core::intrinsics::write_bytes(load_address as *mut u8, 0, section_header.size_in_memory as usize); // intrinsics kullanmak yerine ptr::write_bytes daha yaygın
                         ptr::write_bytes(load_address as *mut u8, 0, section_header.size_in_memory as usize);
                    }
                }
            }
            _ => {
                 printk!("Hata: Bilinmeyen bölüm türü: {}", section_header.type_);
                // Bilinmeyen bölüm türü hata olarak kabul edilebilir veya atlanabilir.
                 return Err(SahneError::InvalidParameter);
            }
        }
        // TODO: Relocations'ı uygula (Statik linkleme kullanılıyorsa genellikle gerekmez)
    }

    // 5. Giriş Noktası Adresini Hesapla
    let entry_point_address = program_base_address + file_header.entry_point_offset as usize;
     printk!("Program belleğe yüklendi: {:#x}, Giriş noktası: {:#x}\n", program_base_address, entry_point_address);


    Ok(LoadedProgram {
        entry_point: entry_point_address,
        program_memory_block: program_memory_block as Box<dyn Any>, // Box'ı dyn Any olarak sakla
        program_memory_ptr: program_memory,
        program_memory_size: total_memory_size,
        arg_memory_block: None, // Argüman belleği henüz hazırlanmadı
    })
}

/// Program argümanlarını (argc, argv) hazırlar ve belleğe kopyalar.
/// Programın ana fonksiyonuna geçirilecek formatı oluşturur.
/// Dönüş değeri: (argc, argv_ptr, ayrılan bellek bloklarını tutan Box).
pub fn prepare_program_args(args: Vec<String>) -> Result<(usize, *const *const u8, Box<dyn Any>), SahneError> {
    let argc = args.len();
    // argv işaretçi dizisi + argüman stringleri için toplam bellek boyutu
    // argv dizisi: argc * usize (her işaretçi için)
    // stringler: Her stringin baytları + null terminator (toplam byte sayısı)
    let argv_array_size = argc * mem::size_of::<*const u8>();
    let total_string_bytes: usize = args.iter().map(|s| s.len() + 1).sum(); // +1 for null terminator

    let total_mem_needed = argv_array_size + total_string_bytes;

    if total_mem_needed == 0 {
        // Argüman yoksa
        return Ok((0, ptr::null(), Box::new(()))); // Boş Box döndür
    }

    // Argümanlar için tek bir bellek bloğu ayır
    // Genellikle argv işaretçi dizisi başta, ardından string verileri gelir.
    // 8 bayt hizalama varsayımı.
    let arg_memory = memory::allocate(total_mem_needed)?; // sahne64::memory::allocate kullan

     if arg_memory.is_null() {
         // printk!("Hata: Argüman belleği tahsis edilemedi ({} bayt).\n", total_mem_needed);
         return Err(SahneError::OutOfMemory);
    }
    let arg_memory_block = unsafe { Box::from_raw(arg_memory as *mut u8) }; // Belleği Box'a sarmala

    let argv_ptr_array = arg_memory as *mut *mut u8; // argv işaretçi dizisinin başlangıcı
    let mut current_string_ptr = unsafe { arg_memory.add(argv_array_size) }; // String verisinin başladığı yer


    // Argüman stringlerini kopyala ve argv dizisini doldur
    for (i, arg) in args.into_iter().enumerate() {
        // argv dizisine geçerli stringin adresini yaz
        unsafe {
            ptr::write(argv_ptr_array.add(i), current_string_ptr);
        }

        // String verisini kopyala (null terminator ile)
        let src_slice = arg.as_bytes();
        let dst_slice = unsafe { slice::from_raw_parts_mut(current_string_ptr, src_slice.len() + 1) }; // +1 for null terminator
        dst_slice[0..src_slice.len()].copy_from_slice(src_slice);
        dst_slice[src_slice.len()] = 0; // Null terminator

        // Bir sonraki string için işaretçiyi ilerlet
        current_string_ptr = unsafe { current_string_ptr.add(src_slice.len() + 1) };
    }

     printk!("Argümanlar hazırlandı: argc {}, argv {:#p}\n", argc, argv_ptr_array);

    Ok((argc, argv_ptr_array as *const *const u8, arg_memory_block as Box<dyn Any>)) // argc, argv pointer'ı ve bellek bloğunu dön
}