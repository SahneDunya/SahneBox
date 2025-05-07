// mm/memory.rs
// Fiziksel Bellek Yönetimi ve Basit Tahsis Edici

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use spin::Mutex; // spin crate'i

// TODO: Fiziksel RAM'in başlangıç adresini ve boyutunu belirleyin.
// Bu bilgiler linker scriptinizden veya donanım belgelerinden gelmelidir.
const PHYS_RAM_START: usize = 0x8000_0000; // Varsayımsal RAM başlangıç adresi (RISC-V'de yaygın)
const PHYS_RAM_SIZE: usize = 2 * 1024 * 1024; // 2 MB

// TODO: Çekirdek kodunuzun, verilerinizin ve yığınınızın linker script tarafından
// RAM'de nereye yerleştirildiğini belirleyin.
// Linker scriptinize "__kernel_end" gibi bir sembol ekleyebilirsiniz.
extern "C" {
    // Çekirdek bölümünün sonunu işaret eden sembol (linker scriptten gelir)
    static __kernel_end: u8;
}

// Heap'in başlayacağı fiziksel adres.
// Çekirdek kodunun bittiği yerden hemen sonra başlayabilir.
const HEAP_START: usize = unsafe { &__kernel_end as *const u8 as usize };

// Heap'in boyutu. Toplam RAM boyutundan çekirdeğin kullandığı alanı çıkarın.
const HEAP_SIZE: usize = PHYS_RAM_START + PHYS_RAM_SIZE - HEAP_START;

// Serbest bellek bloğunu temsil eden yapı.
// Her serbest blok, bir sonraki serbest bloğa işaret eder.
struct FreeListNode {
    size: usize,
    next: Option<*mut FreeListNode>,
}

// Basit Serbest Liste Tahsis Edici Yapısı.
// Bir Mutex içinde tutulur çünkü GlobalAlloc trait'i Sync gerektirir.
struct FreeListAllocator {
    head: Option<*mut FreeListNode>, // Serbest listesinin başı
}

unsafe impl Sync for FreeListAllocator {} // Bu basit tahsis edici Sync'tir (Mutex ile korunur)

impl FreeListAllocator {
    // Boş bir tahsis edici oluşturur. init() ile bellek eklenmelidir.
    const fn new() -> Self {
        FreeListAllocator { head: None }
    }

    // Tahsis ediciye bellek ekler (heap alanını başlatır).
    // İlk başta tüm boş alanı tek bir blok olarak listeye ekler.
    pub unsafe fn init(&mut self, heap_start_addr: usize, heap_size: usize) {
        if heap_size == 0 {
            return; // Boş heap
        }

        // Bellek alanının layout gereksinimlerini karşıladığından emin olun.
        // Serbest liste düğümü kadar veya daha büyük ve hizalanmış olmalı.
        if heap_size < core::mem::size_of::<FreeListNode>() || heap_start_addr % core::mem::align_of::<FreeListNode>() != 0 {
              printk!("Bellek alanı tahsis edici için uygun değil! Başlangıç: {:#x}, Boyut: {}\n", heap_start_addr, heap_size);
             // Hata durumunda init yapılamaz.
             return;
        }


        // Tüm alanı tek bir serbest blok olarak ekle
        let initial_node = heap_start_addr as *mut FreeListNode;
        ptr::write_volatile(initial_node, FreeListNode { size: heap_size, next: None });
        self.head = Some(initial_node);

         printk!("Heap başlatıldı: Başlangıç {:#x}, Boyut {}\n", heap_start_addr, heap_size);
    }

    // Bellek tahsis etme (Layout'a uygun boyutta ve hizalamada)
    unsafe fn allocate(&mut self, layout: Layout) -> *mut u8 {
         printk!("Bellek tahsis isteği: Boyut {}, Hizalama {}\n", layout.size(), layout.align());

        let mut current = &mut self.head;
        while let Some(node_ptr) = *current {
            let node = &mut *node_ptr;
            let node_start_addr = node_ptr as usize;

            // Bu blok, istenen Layout'u karşılamak için kullanılabilir mi kontrol et.
            // Hem boyut (layout.size() kadar yer olmalı) hem de hizalama (blok içinde hizalanmış bir adres bulunmalı) kontrol edilir.
            let allocation_start_offset_in_node = node_start_addr.align_offset(layout.align());
            let allocation_start_addr = node_start_addr + allocation_start_offset_in_node;
            let allocation_end_addr = allocation_start_addr + layout.size();

            if allocation_end_addr <= node_start_addr + node.size {
                // Bu blok yeterli alana sahip ve içinde hizalanmış bir yer var.
                // Bloğu kullanacağız.

                // Bloğun başından tahsis edilen yerden önceki kısım (varsa)
                let prev_free_size = allocation_start_addr - node_start_addr;
                // Tahsis edilen yerden sonraki kısım (varsa)
                let next_free_size = (node_start_addr + node.size) - allocation_end_addr;

                if prev_free_size > 0 && next_free_size > 0 {
                    // Blok üç parçaya ayrılıyor: önceki serbest, tahsis edilen, sonraki serbest
                    // Önceki serbest bloğu güncelle
                    node.size = prev_free_size;
                    // Sonraki serbest bloğu listeye ekle
                    let next_free_node_ptr = allocation_end_addr as *mut FreeListNode;
                     if next_free_size >= core::mem::size_of::<FreeListNode>() && allocation_end_addr % core::mem::align_of::<FreeListNode>() == 0 {
                         ptr::write_volatile(next_free_node_ptr, FreeListNode { size: next_free_size, next: node.next });
                         node.next = Some(next_free_node_ptr); // Önceki bloğun next'i yeni bloğa işaret etsin
                     } else {
                           printk!("Hata: Kalan küçük blok FreeListNode için yeterli değil/hizalanmış değil. Kayıp bellek.\n");
                          // Bu küçük kalan alan kaybolur (fragmentasyon).
                          // node.next değişmez.
                     }


                } else if prev_free_size > 0 {
                    // Blok ikiye ayrılıyor: önceki serbest, tahsis edilen
                    // Önceki serbest bloğun boyutunu güncelle. next'i aynı kalır.
                    node.size = prev_free_size;
                } else if next_free_size > 0 {
                    // Blok ikiye ayrılıyor: tahsis edilen, sonraki serbest
                    // Bu durumda mevcut düğüm tahsis edildiği için listeden çıkarılmalı.
                    // Sonraki serbest blok düğümü, tahsis edilen alanın hemen arkasından başlar.
                    let next_free_node_ptr = allocation_end_addr as *mut FreeListNode;
                     if next_free_size >= core::mem::size_of::<FreeListNode>() && allocation_end_addr % core::mem::align_of::<FreeListNode>() == 0 {
                        ptr::write_volatile(next_free_node_ptr, FreeListNode { size: next_free_size, next: node.next });
                        // Mevcut düğümü listeden çıkar
                        *current = Some(next_free_node_ptr); // current.head veya current.next bir sonraki bloğa işaret etsin
                     } else {
                         // printk!("Hata: Kalan küçük blok FreeListNode için yeterli değil/hizalanmış değil. Kayıp bellek.\n");
                         // Mevcut düğümü listeden çıkar. next'i aynı kalır.
                         *current = node.next;
                     }

                } else {
                    // Tüm blok tahsis ediliyor. Bu düğüm listeden çıkarılmalı.
                    *current = node.next;
                }

                // Tahsis edilen alanın başlangıç adresini döndür.
                 printk!("Tahsis edildi: {:#x}, Boyut: {}\n", allocation_start_addr, layout.size());
                return allocation_start_addr as *mut u8;
            } else {
                // Bu blok uygun değil, bir sonraki bloğa geç
                current = &mut node.next;
            }
        }

        // Uygun serbest blok bulunamadı
         printk!("Tahsis hatası: Yeterli serbest bellek yok.\n");
        ptr::null_mut() // Başarısızlık durumunda null pointer dön
    }

    // Belleği serbest bırakma
    // TODO: Birleştirme (coalescing) implementasyonu eklenmeli.
    // Serbest bırakılan bloğu, serbest listedeki bitişik bloklarla birleştirmek fragmentasyonu azaltır.
    unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
         printk!("Bellek serbest bırakma: {:#p}, Boyut: {}\n", ptr, layout.size());

        if ptr.is_null() { return; } // Null pointer serbest bırakılamaz

        // Serbest bırakılan bloğun bilgisi (geçici olarak FreeListNode gibi davranır)
        // Layout'tan gelen boyut, tahsis edilen gerçek boyutu temsil etmeyebilir
        // Tahsis edicide kaydedilen gerçek boyutu bilmek gerekir (karmaşık)
        // Basitlik için, Layout.size()'ı kullanıyoruz, ama bu doğru birleştirme için yeterli değil.
        // Doğru birleştirme için tahsis edilen bloğun boyutunu bir şekilde takip etmek gerekir.

        // Serbest bırakılan bloğu FreeListNode olarak yaz (geçici olarak)
        let freed_node_ptr = ptr as *mut FreeListNode;
        // Boyut bilgisi burada doğru olmayabilir!
        ptr::write_volatile(freed_node_ptr, FreeListNode { size: layout.size(), next: self.head }); // Listenin başına ekle (basit)

        self.head = Some(freed_node_ptr);

        // TODO: Burada bitişik serbest blokları birleştirme (coalescing) mantığı eklenmeli.
        // Serbest bırakılan bloğun hem önceki hem de sonraki bellek adreslerindeki serbest bloklarla
        // adreslerine göre sıralı listede tutularak birleştirilmesi gerekir.
    }
}

// Global Tahsis Edici örneği (Mutex ile korunur)
#[global_allocator]
static GLOBAL_ALLOCATOR: Mutex<FreeListAllocator> = Mutex::new(FreeListAllocator::new());

// GlobalAlloc trait implementasyonu (GlobalAllocator'ımız için)
unsafe impl GlobalAlloc for Mutex<FreeListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock().allocate(layout) // Kilitli allocator üzerinden tahsis et
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
         self.lock().deallocate(ptr, layout); // Kilitli allocator üzerinden serbest bırak
    }
}

// Bellek yönetimini başlatır. Heap alanını tahsis ediciye ekler.
pub fn init() {
    unsafe {
        // Global tahsis ediciyi başlat.
        // Heap'in başlangıç adresini ve boyutunu geçir.
        GLOBAL_ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
    // printk!("Bellek yönetimi başlatıldı. Heap boyutu: {}\n", HEAP_SIZE);
}

// TODO: Sayfalama (paging) ile ilgili fonksiyonlar buraya eklenebilir
// eğer basit kimlik eşlemesi veya MMU kontrolü yapılacaksa.
 fn enable_paging(...)
 fn create_page_table(...)