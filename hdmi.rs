// drivers/display.rs
// Ekran (HDMI/DisplayPort/Dahili) ve Framebuffer Sürücüsü

use core::slice;
use core::ptr::NonNull;
use spin::Mutex;
use crate::printk;
use crate::rs_io;

// TODO: Ekran denetleyicisinin gerçek MMIO adresini ve register offsetlerini belirleyin.
const DISPLAY_CONTROLLER_BASE_ADDRESS: usize = 0xYYYY_0000; // Varsayımsal
const DISPLAY_WIDTH: usize = 800;
const DISPLAY_HEIGHT: usize = 600;
const PIXEL_SIZE_BYTES: usize = 4; // Örnek: 32-bit renk (ARGB/RGBA)
const FRAMEBUFFER_SIZE: usize = DISPLAY_WIDTH * DISPLAY_HEIGHT * PIXEL_SIZE_BYTES;

// TODO: Framebuffer için bellek adresi. Bu alan çekirdek başlatılırken tahsis edilmeli
// veya linker script ile belirli bir sabit adrese yerleştirilmelidir.
// Bu adres, ekran denetleyicisinin okuyabileceği bir alanda olmalıdır.
const FRAMEBUFFER_PHYS_ADDRESS: usize = 0xZAAA_0000; // Varsayımsal Fiziksel Adres
// TODO: Eğer sanal adresleme kullanılıyorsa, bu adresin çekirdek sanal adres alanındaki eşlemesi bulunmalıdır.
const FRAMEBUFFER_VIRT_ADDRESS: usize = FRAMEBUFFER_PHYS_ADDRESS; // Basitlik için fiziksel == sanal varsayımı


struct Display {
    controller_base: usize,
    framebuffer: Option<NonNull<u8>>, // Framebuffer belleğine işaretçi
    // Diğer durumlar (çözünürlük, renk derinliği, aktif çıkış vb.) eklenebilir
}

impl Display {
    const fn new(controller_base: usize) -> Self {
        Display {
            controller_base,
            framebuffer: None,
        }
    }

    // Ekran donanımını başlatır (mod, çözünürlük, framebuffer adresi vb. ayarları).
    // TODO: Gerçek donanıma göre doldurun.
    pub fn init(&mut self) {
        // TODO: Framebuffer belleği için bir işaretçi al veya tahsis et (alloc?)
        // Şimdilik sabit adresi kullanıyoruz, alloc gerekebilir eğer dinamikse.
        self.framebuffer = NonNull::new(FRAMEBUFFER_VIRT_ADDRESS as *mut u8);

        // TODO: Ekran denetleyicisini yapılandır:
        // 1. Çözünürlük ve zamanlama sinyalleri (800x600). Bu VESA CVT gibi standartlar veya donanıma özel timing registerları gerektirir.
        // 2. Piksel formatı ve renk derinliği ayarı.
        // 3. Framebuffer'ın fiziksel adresini denetleyiciye yaz.
        // unsafe { rs_io::mmio_write32(self.controller_base + FRAMEBUFFER_ADDR_REG_OFFSET, FRAMEBUFFER_PHYS_ADDRESS as u32); } // 32-bit MMIO varsayımı
        // 4. Ekran çıkışını etkinleştir (HDMI veya DisplayPort'u seçme ve aktif etme).
         unsafe { rs_io::mmio_write32(self.controller_base + CONTROL_REG_OFFSET, ENABLE_BIT | HDMI_SELECT_BIT); }

        if self.framebuffer.is_some() {
            printk!("Ekran sürücüsü başlatıldı. Framebuffer adresi: {:#p}\n", self.framebuffer.unwrap());
            // TODO: Framebuffer'ı temizle (örneğin siyah renge boya)
             if let Some(fb_ptr) = self.framebuffer {
                 unsafe { fb_ptr.as_ptr().write_bytes(0, FRAMEBUFFER_SIZE); } // Siyah (varsayımsal 0 renk değeri)
             }
        } else {
             printk!("Ekran sürücüsü başlatılamadı: Framebuffer bellek adresi geçersiz.\n");
        }
    }

    // Framebuffer belleğine mutable bir slice olarak erişim sağlar.
    // Üst katmanlar bu slice'a yazarak ekrana çizim yapar.
    // Bu slice, doğrudan donanımın okuduğu belleği temsil eder.
    pub fn framebuffer(&mut self) -> Option<&mut [u8]> {
        self.framebuffer.map(|ptr| {
            unsafe { slice::from_raw_parts_mut(ptr.as_ptr(), FRAMEBUFFER_SIZE) }
        })
    }

    // Ekran boyutu bilgilerini döndürür.
    pub fn resolution(&self) -> (usize, usize) {
        (DISPLAY_WIDTH, DISPLAY_HEIGHT)
    }

    // Piksel boyutu bilgisini döndürür.
    pub fn pixel_size(&self) -> usize {
        PIXEL_SIZE_BYTES
    }
}

// Ekran sürücüsünü korumak için global Mutex
static DISPLAY_DRIVER: Mutex<Display> = Mutex::new(Display::new(DISPLAY_CONTROLLER_BASE_ADDRESS));

// Sürücüyü başlatmak için dışarıdan çağrılacak fonksiyon
pub fn init() {
    DISPLAY_DRIVER.lock().init();
}

// Framebuffer'a erişim sağlayan fonksiyon
pub fn framebuffer() -> Option<&'static mut [u8]> {
    // Mutex'i kilitle ve framebuffer slice'ını al.
    // 'static lifetime, döndürülen referansın programın tamamı boyunca geçerli olabileceğini belirtir.
    // Bu dikkatli kullanılmalıdır, MutexGuard scope'dan çıkınca kilit serbest bırakılır,
    // ancak slice referansı hala geçerli olabilir (bu bir sorun olabilir!).
    // Daha güvenli yaklaşım, bu fonksiyonun bir MutexGuard döndürmesi ve slice'a Guard üzerinden erişilmesidir.
    // Basitlik için slice döndürüyoruz, ama dikkatli olun.
    let mut driver = DISPLAY_DRIVER.lock();
    driver.framebuffer().map(|slice| unsafe { &mut *(slice as *mut [u8]) })
}

// Ekran çözünürlüğünü döndürür
pub fn resolution() -> (usize, usize) {
    DISPLAY_DRIVER.lock().resolution()
}

// Piksel boyutunu döndürür
pub fn pixel_size() -> usize {
    DISPLAY_DRIVER.lock().pixel_size()
}

// TODO: Dokunmatik ekran kısmı ayrı bir sürücü (touchscreen.rs) olarak ele alınacaktır.