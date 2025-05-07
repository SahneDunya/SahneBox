// filesystem/ext.rs
// Minimal EXT2 Dosya Sistemi Sürücüsü (Sadece Okuma)
// Çekirdekten block device Handle'ını kullanarak çalışır.

#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz (Kullanıcı alanı kütüphanesi)
#![allow(dead_code)] // Henüz kullanılmayan kodlar için uyarı vermesin
#![feature(alloc)] // Box ve Vec kullanmak için alloc feature'ı

extern crate alloc; // Heap tahsisi için alloc crate'ini kullan

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem;
use core::str;

// Çekirdek API'mızı içeri aktarıyoruz
use crate::sahne64::{self, resource, memory, SahneError, Handle};


// EXT2 Sabitleri
const EXT2_SUPERBLOCK_BLOCK: u64 = 1; // Süper blok genellikle 1. blokta başlar (1024 bayt bloklar için)
const EXT2_SUPERBLOCK_OFFSET: usize = 1024; // 512 bayt sektörlerde 1024 offset (yani 2. sektör)
const EXT2_MIN_BLOCK_SIZE: u32 = 1024; // EXT2 minimum blok boyutu 1KB (2^10)
const EXT2_SUPERBLOCK_MAGIC: u16 = 0xEF53; // Süper blok sihirli sayısı

// EXT2 i-node modları (Basit örnekler)
const S_IFREG: u16 = 0x8000; // Normal dosya
const S_IFDIR: u16 = 0x4000; // Dizin

// On-disk EXT2 yapıları için Rust temsilleri
// Bunlar doğrudan blok cihazdan okunacak bayt yapısına karşılık gelmeli.
// Alan boyutları ve offsetleri EXT2 spesifikasyonuna göre ayarlanmalıdır.
// Little-endian varsayılmıştır. #[repr(C)] ve #[packed] gerekebilir.

#[repr(C, packed)] // C uyumluluğu ve sıkı paketleme (alanlar arasında boşluk olmamalı)
struct Superblock {
    s_inodes_count: u32, // i-node sayısı
    s_blocks_count: u32, // Blok sayısı
    s_r_blocks_count: u32, // Ayrılmış blok sayısı
    s_free_blocks_count: u32, // Serbest blok sayısı
    s_free_inodes_count: u32, // Serbest i-node sayısı
    s_first_data_block: u32, // İlk veri bloğu (genellikle 0 veya 1)
    s_log_block_size: u32, // Blok boyutu = 1024 << s_log_block_size
    s_log_frag_size: u32, // Fragment boyutu (basit implementasyonda ignore edilebilir)
    s_blocks_per_group: u32, // Blok grubundaki blok sayısı
    s_frags_per_group: u32, // Blok grubundaki fragment sayısı
    s_inodes_per_group: u32, // Blok grubundaki i-node sayısı
    s_mtime: u32, // Son bağlama zamanı
    s_wtime: u32, // Son yazma zamanı
    s_mnt_count: u16, // Bağlama sayısı
    s_max_mnt_count: u16, // Maksimum bağlama sayısı
    s_magic: u16, // Sihirli sayı (EXT2_SUPERBLOCK_MAGIC)
    s_state: u16, // Dosya sistemi durumu
    s_errors: u16, // Hata davranışı
    s_minor_rev_level: u16, // Küçük Revizyon Seviyesi
    s_lastcheck: u32, // Son kontrol zamanı
    s_checkinterval: u32, // Kontrol aralığı
    s_creator_os: u32, // Oluşturan OS
    s_rev_level: u32, // Revizyon seviyesi
    s_def_resuid: u16, // Varsayılan kullanıcı id'si
    s_def_resgid: u16, // Varsayılan grup id'si
    // ... Süper bloğun geri kalanı (toplam 1024 bayt)
    // UUID, Volume Name, Last Mounted, vb.
}

#[repr(C, packed)]
struct GroupDescriptor {
    bg_block_bitmap: u32, // Blok bitmap bloğunun konumu
    bg_inode_bitmap: u32, // i-node bitmap bloğunun konumu
    bg_inode_table: u32, // i-node tablosu bloğunun konumu
    bg_free_blocks_count: u16, // Blok grubundaki serbest blok sayısı
    bg_free_inodes_count: u16, // Blok grubundaki serbest i-node sayısı
    bg_used_dirs_count: u16, // Blok grubundaki kullanılan dizin sayısı
    bg_pad: u16, // Hizalama için
    bg_reserved: [u32; 3], // Gelecek kullanım için
}

#[repr(C, packed)]
struct Inode {
    i_mode: u16, // Dosya tipi ve izinler
    i_uid: u16, // Sahibi kullanıcı id'si
    i_size: u32, // Dosya boyutu (düşük 32 bit)
    i_atime: u32, // Erişim zamanı
    i_ctime: u32, // Oluşturma zamanı
    i_mtime: u32, // Değiştirme zamanı
    i_dtime: u32, // Silme zamanı
    i_gid: u16, // Sahibi grup id'si
    i_links_count: u16, // Bağlantı (hard link) sayısı
    i_blocks: u32, // Tahsis edilen blok sayısı (512 bayt biriminde!)
    i_flags: u32, // Bayraklar
    // OSPTR1 (işletim sistemine özel)
    // i_block: [u32; 15], // Blok işaretçileri (0-11 doğrudan, 12 tek dolaylı, 13 çift dolaylı, 14 üç dolaylı)
    i_block: [u32; 15], // Minimal için sadece doğrudan işaretçilerle ilgileneceğiz
    i_generation: u32, // Dosya sürümü (NFS için)
    i_file_acl: u32, // Dosya ACL'si (yüksek 32 bit i_size'ın parçası olabilir EXT2 revizyona göre)
    i_dir_acl: u32, // Dizin ACL'si (yüksek 32 bit i_size'ın parçası olabilir)
    i_faddr: u32, // Fragment adresi
    // OSPTR2 (işletim sistemine özel)
    // i_size_high (yüksek 32 bit, 4GB'tan büyük dosyalar için)
    // i_author (yüksek 32 bit i_uid/i_gid olabilir)
}

#[repr(C, packed)]
struct DirectoryEntry {
    inode: u32, // i-node numarası
    rec_len: u16, // Kayıt uzunluğu
    name_len: u8, // İsim uzunluğu (EXT2 revizyon 0'da u8, revizyon 1'de 16384 >> s_log_block_size)
    file_type: u8, // Dosya tipi (EXT2 revizyon 1 ve sonrası)
    name: [u8; 255], // Dosya ismi (maksimum 255 bayt, rec_len'e göre değişir)
}


// Dosya Sistemi Ana Yapısı
pub struct ExtFilesystem {
    device_handle: Handle, // Blok cihaza erişim için çekirdek Handle'ı
    block_size: u32, // Dosya sistemi blok boyutu (genellikle 1024, 2048, 4096)
    inode_size: u16, // i-node boyutu (128 veya 256)
    blocks_per_group: u32,
    inodes_per_group: u32,
    first_data_block: u32,
    total_inode_count: u32,
    total_block_count: u32,
    group_count: u32,
    group_descriptors: Vec<GroupDescriptor>, // Tüm blok gruplarının tanımlayıcıları
    // Diğer süper blok bilgileri eklenebilir
}

impl ExtFilesystem {
    /// Belirtilen blok cihazı Handle'ından EXT2 dosya sistemini bağlar (mount).
    /// Sadece okuma amaçlıdır.
    pub fn mount(device_handle: Handle) -> Result<Self, SahneError> {
        if !device_handle.is_valid() {
            return Err(SahneError::InvalidHandle);
        }

        // Süper bloğu oku (genellikle blok 1, offset 1024)
        // Blok 1, sektör 2'ye denk gelir (sektör boyutu 512 varsayımıyla).
        let mut super_block_buffer = alloc::vec![0u8; 1024]; // Süper blok boyutu 1024
        // Blok cihazdan 2. sektörden (offset 1024) 1024 bayt oku
        let bytes_read = resource::read(device_handle, &mut super_block_buffer, EXT2_SUPERBLOCK_OFFSET, super_block_buffer.len())?;
        if bytes_read != super_block_buffer.len() {
             return Err(SahneError::InvalidOperation); // Yeterli veri okunamadı
        }

        // Süper bloğu parse et
        // Okunan baytları Superblock yapısına pointer casting ile eriş
        let superblock: &Superblock = unsafe {
            // Süper bloğun başlangıç adresini al ve Superblock pointer'a dönüştür
            let ptr = super_block_buffer.as_ptr() as *const Superblock;
            // Dereference et
            &*ptr
        };

        // Süper blok sihirli sayısını kontrol et
        if superblock.s_magic != EXT2_SUPERBLOCK_MAGIC {
            sahne64::printk!("EXT: Geçersiz sihirli sayı: {:#x}\n", superblock.s_magic);
            return Err(SahneError::InvalidParameter); // Veya NamingError / InvalidFormat
        }

        let block_size = EXT2_MIN_BLOCK_SIZE << superblock.s_log_block_size;
        let inode_size = if superblock.s_rev_level >= 1 { superblock.s_inode_size } else { 128 }; // Rev 0 ise 128

        sahne64::printk!("EXT: Dosya sistemi bulundu! Blok boyutu: {}, i-node boyutu: {}\n", block_size, inode_size);
         printk!("EXT: Toplam i-node: {}, Toplam blok: {}\n", superblock.s_inodes_count, superblock.s_blocks_count);


        // Blok grubunun kaç blok içerdiğini hesapla (Süper bloktan s_blocks_per_group kullanılır)
        let blocks_per_group = superblock.s_blocks_per_group;
        let inodes_per_group = superblock.s_inodes_per_group;
        let first_data_block = superblock.s_first_data_block;
        let total_inode_count = superblock.s_inodes_count;
        let total_block_count = superblock.s_blocks_count;

        // Blok grubu sayısını hesapla
        let group_count = (total_block_count + blocks_per_group - 1) / blocks_per_group;
         printk!("EXT: Blok grubu sayısı: {}\n", group_count);

        // Grup Tanımlayıcı Tablosunu oku
        // Grup tanımlayıcı tablosu, ilk veri bloğundan sonra başlar.
        // 1024 bayt bloklar için genellikle blok 2'de başlar.
        // 2048/4096 bayt bloklar için genellikle blok 1'de başlar.
        let group_desc_block = superblock.s_first_data_block + 1; // Genellikle blok 2
        let group_desc_size = mem::size_of::<GroupDescriptor>();
        let group_desc_table_size = (group_count as usize) * group_desc_size;
        let mut group_desc_buffer = alloc::vec![0u8; group_desc_table_size];

        // Blok cihazdan Grup Tanımlayıcı Tablosu'nu oku
        // EXT2 dosya sistemi bloklarını cihaz bloklarına çevirmek gerek!
        // EXT2 blok X -> Cihaz bloğu (X * block_size) / device_block_size
        // Basitlik için cihaz blok boyutu 512 bayt varsayalım.
        let device_block_size = 512; // TODO: Kernelden veya donanımdan öğrenilmeli!
        let group_desc_device_block_start = (group_desc_block as u64 * block_size as u64) / device_block_size as u64;
        let group_desc_offset_in_device_block = (group_desc_block as u64 * block_size as u64) % device_block_size as u64;

        let bytes_read = resource::read(
             device_handle,
             &mut group_desc_buffer,
             group_desc_device_block_start as usize * device_block_size + group_desc_offset_in_device_block as usize, // Okuma offseti
             group_desc_buffer.len() // Okunacak uzunluk
        )?;

         if bytes_read != group_desc_buffer.len() {
             return Err(SahneError::InvalidOperation); // Yeterli veri okunamadı
         }


        // Grup tanımlayıcılarını parse et
        let group_descriptors: Vec<GroupDescriptor> = (0..group_count)
            .map(|i| {
                let offset = i as usize * group_desc_size;
                let ptr = unsafe { group_desc_buffer.as_ptr().add(offset) as *const GroupDescriptor };
                unsafe { ptr.read_unaligned() } // Paketlenmiş yapıları okurken unaligned read gerekebilir
            })
            .collect();

         printk!("EXT: {} blok grubu tanımlayıcısı okundu.\n", group_descriptors.len());
         printk!("EXT: İlk grup i-node tablosu: Blok {}\n", group_descriptors[0].bg_inode_table);


        Ok(ExtFilesystem {
            device_handle,
            block_size,
            inode_size,
            blocks_per_group,
            inodes_per_group,
            first_data_block,
            total_inode_count,
            total_block_count,
            group_count,
            group_descriptors,
        })
    }

    /// Blok numaralarını cihaz bloklarına çeviren yardımcı fonksiyon.
    /// EXT2 blokları -> cihaz offseti (bayt cinsinden)
    fn fs_block_to_device_offset(&self, fs_block: u32) -> usize {
         let device_block_size = 512; // TODO: Kernelden veya donanımdan öğrenilmeli!
         (fs_block as u64 * self.block_size as u64) as usize
         // Eğer cihazın blok boyutu farklıysa burası değişir.
         // Örneğin, cihaz 512 bayt, FS 4096 bayt blok kullanıyorsa:
         // (fs_block * 4096) / 512 = fs_block * 8 cihaz bloğu
         // Yani offset = fs_block * 8 * 512
         // En basiti: offset = fs_block * self.block_size
    }


    /// i-node numarasından i-node yapısını okur.
    /// Root i-node genellikle 2 numaralı i-node'dur.
    pub fn read_inode(&self, inode_number: u32) -> Result<Inode, SahneError> {
        if inode_number == 0 || inode_number > self.total_inode_count {
            return Err(SahneError::InvalidParameter); // Geçersiz i-node numarası
        }

        // i-node'un ait olduğu blok grubunu ve grubun içindeki index'ini hesapla
        let inode_index_in_fs = inode_number - 1; // i-node numaraları 1 tabanlı
        let group_index = inode_index_in_fs / self.inodes_per_group;
        let inode_index_in_group = inode_index_in_fs % self.inodes_per_group;

        if group_index >= self.group_count {
            return Err(SahneError::InvalidParameter); // Hesaplanan grup geçersiz
        }

        // Grup tanımlayıcısını al
        let group_desc = &self.group_descriptors[group_index as usize];

        // i-node tablosunun başlangıç bloğunu al
        let inode_table_start_block = group_desc.bg_inode_table;

        // i-node'un tablo içindeki ofsetini hesapla (bayt cinsinden)
        let inode_offset_in_table = inode_index_in_group as usize * self.inode_size as usize;

        // i-node'un bulunduğu dosya sistemi bloğunu hesapla
        let inode_fs_block = inode_table_start_block + (inode_offset_in_table as u32 / self.block_size);

        // i-node'un bulunduğu blok içindeki ofsetini hesapla (bayt cinsinden)
        let inode_offset_in_block = inode_offset_in_table % self.block_size as usize;

        // i-node bloğunu cihazdan oku
        let mut block_buffer = alloc::vec![0u8; self.block_size as usize];
        let device_offset = self.fs_block_to_device_offset(inode_fs_block);

        let bytes_read = resource::read(self.device_handle, &mut block_buffer, device_offset, self.block_size as usize)?;
        if bytes_read != self.block_size as usize {
            return Err(SahneError::InvalidOperation); // Blok okunamadı
        }

        // i-node yapısını buffer'dan al
        let inode: Inode = unsafe {
            let ptr = block_buffer.as_ptr().add(inode_offset_in_block) as *const Inode;
            ptr.read_unaligned() // Paketlenmiş yapıları okurken unaligned read gerekebilir
        };

        Ok(inode)
    }

    /// Bir dizinin içeriğini listeler (alt dosya ve dizin isimleri).
    /// Sadece EXT2 revizyon 0 veya 1'deki temel dizin girişlerini destekler.
    pub fn list_directory(&self, dir_inode: &Inode) -> Result<Vec<DirectoryEntry>, SahneError> {
        if (dir_inode.i_mode & S_IFDIR) == 0 {
            return Err(SahneError::InvalidOperation); // Bu bir dizin değil
        }

        // Dizin içeriği i_block işaretçilerindeki veri bloklarında saklanır.
        // Sadece doğrudan (direct) blok işaretçilerini destekliyoruz (i_block[0] - i_block[11]).
        let mut entries = Vec::new();
        let mut current_offset_in_dir = 0;

        // TODO: Dolaylı (indirect) blok işaretçileri (i_block[12], [13], [14]) implemente edilmeli
        // eğer dizin içeriği birden fazla doğrudan bloktan büyükse.
        // Bu implementasyon sadece i_block[0]'daki veriyi okur.
        let data_block_number = dir_inode.i_block[0]; // İlk doğrudan blok

        if data_block_number == 0 {
             return Ok(entries); // Boş dizin
        }

        let mut block_buffer = alloc::vec![0u8; self.block_size as usize];
        let device_offset = self.fs_block_to_device_offset(data_block_number);

        let bytes_read = resource::read(self.device_handle, &mut block_buffer, device_offset, self.block_size as usize)?;
        if bytes_read != self.block_size as usize {
            return Err(SahneError::InvalidOperation); // Dizin bloğu okunamadı
        }

        // Blok içindeki dizin girişlerini parse et
        while current_offset_in_dir < self.block_size as usize {
            let entry: &DirectoryEntry = unsafe {
                let ptr = block_buffer.as_ptr().add(current_offset_in_dir) as *const DirectoryEntry;
                &*ptr // Dereference
                // read_unaligned() gerekebilir eğer packed struct kullanıyorsak?
                 ptr.read_unaligned()
            };

            // Geçersiz giriş (inode 0 ise) veya son giriş (rec_len çok büyükse)
            if entry.inode == 0 || entry.rec_len == 0 {
                 // Bu girişi atla, genellikle son giriş veya silinmiş girişler böyle olur
                 // Ama döngüyü kırmamak için rec_len kadar ilerlemek önemli.
                 if entry.rec_len > 0 {
                      current_offset_in_dir += entry.rec_len as usize;
                      continue;
                 } else {
                      // rec_len 0 ise sonsuz döngüyü önle
                      break;
                 }
            }


            // İsim uzunluğunu kontrol et (name_len 255'ten büyük olamaz)
            let name_len = entry.name_len as usize;
            if name_len > 255 || (current_offset_in_dir + entry.rec_len as usize) > self.block_size as usize {
                  printk!("EXT: Hata: Geçersiz dizin girişi veya rec_len! Offset: {}, rec_len: {}\n", current_offset_in_dir, entry.rec_len);
                 break; // Geçersiz yapı, döngüyü kır
            }

            // İsim slice'ını al ve String'e dönüştür
            let name_slice = &entry.name[0..name_len];
            let name = str::from_utf8(name_slice).unwrap_or("<geçersiz isim>"); // UTF8 değilse placeholder

             printk!("EXT: Dizin Girişi: i-node {}, rec_len {}, name_len {}, name: {}\n", entry.inode, entry.rec_len, entry.name_len, name);


            // Yeni bir DirectoryEntry struct oluştur ve listeye ekle
            // Kendi hafıza alanına kopyalamak daha güvenlidir.
            let mut owned_entry = DirectoryEntry {
                inode: entry.inode,
                rec_len: entry.rec_len,
                name_len: entry.name_len,
                name: [0u8; 255], // Varsayılan sıfırlarla doldur
                file_type: entry.file_type, // Rev 1+ için
            };
            owned_entry.name[0..name_len].copy_from_slice(name_slice);

            entries.push(owned_entry);


            // Bir sonraki girişe geç
            current_offset_in_dir += entry.rec_len as usize;
        }

        Ok(entries)
    }


    /// Bir dosyadan veri okur.
    /// Sadece doğrudan (direct) blok işaretçilerini destekler ve dosya boyutuyla sınırlıdır.
    pub fn read_file(&self, file_inode: &Inode, buffer: &mut [u8], offset: usize) -> Result<usize, SahneError> {
        if (file_inode.i_mode & S_IFREG) == 0 {
            return Err(SahneError::InvalidOperation); // Bu bir dosya değil
        }

        let file_size = file_inode.i_size as usize; // Sadece düşük 32 bit (4GB'tan büyük dosyalar desteklenmez)
        let mut bytes_to_read = buffer.len();
        let mut current_offset_in_file = offset;
        let mut buffer_write_offset = 0;

        // Okunacak byte sayısını dosya boyutu ve offset ile sınırla
        if current_offset_in_file >= file_size {
            return Ok(0); // Offset dosya boyutundan büyük veya eşit
        }
        if bytes_to_read > file_size - current_offset_in_file {
            bytes_to_read = file_size - current_offset_in_file;
        }

        let initial_bytes_to_read = bytes_to_read;

        // TODO: Dolaylı (indirect) blok işaretçileri (i_block[12], [13], [14]) implemente edilmeli
        // eğer dosya boyutu birden fazla doğrudan bloktan büyükse.
        // Bu implementasyon sadece i_block[0] - i_block[11]'deki verilere doğrudan erişir.

        while bytes_to_read > 0 && current_offset_in_file < file_size {
            // Hangi dosya sistemi bloğunda olduğumuzu hesapla
            let fs_block_index_in_file = current_offset_in_file / self.block_size as usize;
            let offset_in_fs_block = current_offset_in_file % self.block_size as usize;
            let bytes_left_in_block = self.block_size as usize - offset_in_fs_block;
            let read_len_in_block = core::cmp::min(bytes_to_read, bytes_left_in_block);

            // İlgili blok numarasını al (sadece doğrudan işaretçiler)
            if fs_block_index_in_file >= file_inode.i_block.len() {
                 printk!("EXT: Hata: Dosya boyutu doğrudan blokları aştı ama dolaylı bloklar implemente edilmedi.\n");
                break; // Desteklenmeyen dosya yapısı
            }
            let data_fs_block = file_inode.i_block[fs_block_index_in_file];

            if data_fs_block == 0 {
                  printk!("EXT: Hata: Okunmaya çalışılan blok (index {}) boş (0). Dosya sparse olabilir veya hata var.\n", fs_block_index_in_file);
                 // Boş bloktan okunuyorsa (sparse dosya), sıfır bayt döndürülür.
                 // Buffer'ın ilgili kısmını sıfırla
                 for i in 0..read_len_in_block {
                     buffer[buffer_write_offset + i] = 0;
                 }
            } else {
                 // Veri bloğunu cihazdan oku
                 let device_offset = self.fs_block_to_device_offset(data_fs_block);
                 let mut block_buffer = alloc::vec![0u8; self.block_size as usize]; // Geçici buffer
                 let bytes_read_from_block = resource::read(self.device_handle, &mut block_buffer, device_offset, self.block_size as usize)?;

                 if bytes_read_from_block != self.block_size as usize {
                     return Err(SahneError::InvalidOperation); // Blok okunamadı
                 }

                 // Okunan veriyi hedef buffera kopyala
                 buffer[buffer_write_offset .. buffer_write_offset + read_len_in_block]
                    .copy_from_slice(&block_buffer[offset_in_fs_block .. offset_in_fs_block + read_len_in_block]);
            }


            // Offsetleri ve kalan okunacak byte sayısını güncelle
            current_offset_in_file += read_len_in_block;
            buffer_write_offset += read_len_in_block;
            bytes_to_read -= read_len_in_block;
        }

        Ok(initial_bytes_to_read - bytes_to_read) // Gerçekten okunan toplam byte sayısı
    }


    // TODO: Başka temel dosya sistemi fonksiyonları (read_link, stat, vb.) eklenebilir.
    // TODO: Dolaylı (indirect) blok işaretçileri için yardımcı fonksiyonlar yazılmalı.
     pub fn resolve_indirect_block(&self, block_number: u32, indirect_level: u32) -> Result<Vec<u32>, SahneError> { ... }

}


// Helper fonksiyon: resource::read için offset ekleyen versiyon
// Bu, çekirdeğin resource::read API'sinde offset parametresi olduğunu varsayar.
// Eğer yoksa, çekirdek API'si güncellenmeli veya dosya sistemi blokları
// doğrudan cihazın sektörlerine (örn. 512 bayt) dönüştürülmeli.
impl resource::resource_impl { // resource modülü içinde bir impl bloğu varsayalım
    // resource::read fonksiyonunu offset parametresi ile genişleten wrapper
    // Offset cihazın başlangıcına göredir.
    pub fn read(handle: Handle, buffer: &mut [u8], offset: usize, len: usize) -> Result<usize, SahneError> {
        // sahne64::syscall(arch::SYSCALL_RESOURCE_READ, handle.raw(), buf_ptr, buf_len, offset as u64, 0) gibi olmalı
        // Mevcut Sahne64 resource::read API'sinde offset yok.
        // API ya güncellenmeli ya da dosya sistemi offseti cihaza özgü sektörlere çevirmeli.
        // Şimdilik, mevcut API'yi kullanıp, offset'i arg4 olarak geçirecek şekilde syscall'ı doğrudan çağıralım.
        // BU, sahne64::resource::read'in iç implementasyonunu 'hacklemek' demektir!
        // İdeal olan sahne64::resource::read API'sinin offset parametresi içermesidir.

        // Sahne64 API'sini bu senaryo için genişletilmiş varsayalım:
          pub fn read_at(handle: Handle, buffer: &mut [u8], offset: usize) -> Result<usize, SahneError>

        // Eğer resource::read API'sinde offset yoksa, aşağıdaki gibi doğrudan syscall çağrılabilir:
        let buffer_ptr = buffer.as_mut_ptr() as u64;
        let buffer_len = buffer.len() as u64;
        let result = unsafe {
            sahne64::syscall(
                sahne64::arch::SYSCALL_RESOURCE_READ,
                handle.raw(), // arg1: handle
                buffer_ptr,   // arg2: buffer ptr
                buffer_len,   // arg3: buffer len
                offset as u64,// arg4: offset (yeni argüman)
                len as u64    // arg5: len (yeni argüman) - Veya len buffer_len ile aynıysa arg3 yeterli
            )
        };

        if result < 0 {
             Err(sahne64::map_kernel_error(result))
        } else {
             Ok(result as usize)
        }

        // TODO: resource::read API'si çekirdek tarafında offset'i destekleyecek şekilde güncellenmeli.
        // O zaman bu helper fonksiyonun yerine doğrudan resource::read_at çağrılabilir.
    }

    // resource::write fonksiyonunu offset ekleyen versiyon (read ile benzer)
     #[allow(dead_code)]
     pub fn write(handle: Handle, buffer: &[u8], offset: usize, len: usize) -> Result<usize, SahneError> {
         let buffer_ptr = buffer.as_ptr() as u64;
         let buffer_len = buffer.len() as u64; // len argümanını kullanmıyoruz, buffer.len() kullanıyoruz.
         let result = unsafe {
             sahne64::syscall(
                 sahne64::arch::SYSCALL_RESOURCE_WRITE,
                 handle.raw(), // arg1: handle
                 buffer_ptr,   // arg2: buffer ptr
                 buffer_len,   // arg3: buffer len
                 offset as u64,// arg4: offset (yeni argüman)
                  len as u64 // arg5: len (isteğe bağlı)
                 0 // arg5 kullanılmıyorsa 0
             )
         };

        if result < 0 {
             Err(sahne64::map_kernel_error(result))
        } else {
             Ok(result as usize)
        }
        // TODO: resource::write API'si çekirdek tarafında offset'i destekleyecek şekilde güncellenmeli.
     }

}


// Root dizinine erişmek için kolaylık fonksiyonu
impl ExtFilesystem {
     pub fn root_directory(&self) -> Result<Inode, SahneError> {
         self.read_inode(2) // EXT2'de root i-node her zaman 2'dir.
     }
}