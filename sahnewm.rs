// windows_system/sahnewm/src/main.rs
// SahneBox Pencereleme Sunucusu / Yöneticisi (sahnewm)

#![no_std]
#![feature(alloc)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;
use core::fmt::Write;
use core::slice;
use core::ptr;


// SahneBox Çekirdek API'sini içeri aktar
use crate::sahne64::{self, resource, memory, task, SahneError, Handle};
// Çekirdek display ve touchscreen sürücülerine (veya onların resource arayüzlerine) erişim gereklidir.
// Varsayalım ki resource::acquire ile erişilebiliyor.
 use crate::drivers::display; // Eğer syscall yerine driver'a doğrudan erişim varsa (pek olası değil)
 use crate::drivers::touchscreen; // Eğer syscall yerine driver'a doğrudan erişim varsa (pek olası değil)


// TODO: IPC mekanizması için ilgili sahne64::ipc modülünü içeri aktar
 use crate::sahne64::ipc;


// Basit Konsol Yazıcı (Hata ayıklama için)
struct ConsoleWriter { handle: Handle, }
impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        resource::write(self.handle, s.as_bytes(), 0, s.as_bytes().len()).unwrap_or(0);
        Ok(())
    }
}


// Pencereyi temsil eden yapı
struct Window {
    id: u32, // Benzersiz pencere ID'si
    x: i32, y: i32, // Konum
    width: u32, height: u32, // Boyut
    // TODO: Pencere içeriği için bellek tamponu.
    // Bu bellek, istemci uygulama ile paylaşılan bellek olabilir.
    // SharedMemoryHandle ve bu belleğin sunucu tarafındaki eşlenmiş adresi.
    content_buffer: Option<ptr::NonNull<u8>>,
    buffer_size: usize,
    // TODO: İstemci uygulama ile iletişim için IPC endpoint/handle'ı
    // client_endpoint: ipc::Connection?, // Varsayımsal IPC bağlantısı
    z_order: u32, // Z-sırası (hangi pencere üstte)
    // TODO: Pencere durumu (görünür, gizli, minimize vb.)
    // TODO: Pencere başlığı string'i
}

impl Window {
    // Pencerenin ekran üzerindeki alanını kontrol eder.
    fn contains_point(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < (self.x + self.width as i32) &&
        py >= self.y && py < (self.y + self.height as i32)
    }

    // Pencere içeriği tamponuna slice olarak erişim (Eğer paylaşımlı bellek kullanılıyorsa)
    fn get_buffer_slice(&mut self) -> Option<&mut [u8]> {
        self.content_buffer.map(|ptr| {
            unsafe { slice::from_raw_parts_mut(ptr.as_ptr(), self.buffer_size) }
        })
    }

    // TODO: Pencere tamponundan ana frambuffer'a kopyalama fonksiyonu
    // Bu, compositing'in bir parçasıdır.
    fn composite_to_framebuffer(&self, framebuffer: &mut [u8], fb_width: u32, fb_height: u32, fb_pixel_size: u32) {
        if let Some(content) = self.get_buffer_slice() {
            let window_pixel_size = fb_pixel_size; // Basitlik için pencere formatı FB formatı ile aynı varsayalım
            // TODO: Renk derinliğine göre doğru bayt kopyalama yapılmalı.

            for y_win in 0..self.height {
                let y_fb = self.y as u32 + y_win;
                if y_fb >= fb_height { continue; } // Ekran dışı

                for x_win in 0..self.width {
                     let x_fb = self.x as u32 + x_win;
                     if x_fb >= fb_width { continue; } // Ekran dışı

                     let fb_index = ((y_fb * fb_width + x_fb) * fb_pixel_size) as usize;
                     let win_index = ((y_win * self.width + x_win) * window_pixel_size) as usize;

                     // Pikseli pencere tamponundan ana framebuffer'a kopyala
                     if win_index + window_pixel_size as usize <= content.len() && fb_index + fb_pixel_size as usize <= framebuffer.len() {
                         // TODO: Renk derinliğine göre doğru sayıda bayt kopyala.
                         // Örneğin 32-bit renk için 4 bayt.
                         framebuffer[fb_index .. fb_index + window_pixel_size as usize]
                            .copy_from_slice(&content[win_index .. win_index + window_pixel_size as usize]);
                     }
                }
            }
        }
    }
}


// SahneBox Pencereleme Sunucusu Ana Yapısı
struct SahneWindowManager {
    // TODO: Çekirdek ekran kaynağı Handle'ı
    display_handle: Handle,
    // TODO: Çekirdek dokunmatik ekran kaynağı Handle'ı
    touchscreen_handle: Handle,
    // TODO: Ana framebuffer belleğine erişim.
    // Bu, display_handle üzerinden veya doğrudan kernelden alınır.
    framebuffer: Option<ptr::NonNull<u8>>,
    fb_width: u32, fb_height: u32, fb_pixel_size: u32,
    fb_size: usize,

    windows: Vec<Window>, // Yönetilen pencerelerin listesi
    next_window_id: u32, // Yeni pencere ID'si için sayıcı

    // TODO: IPC sunucu endpoint'i
     server_endpoint: ipc::Listener?, // Varsayımsal IPC dinleyici
}

impl SahneWindowManager {
    /// Yeni bir Pencere Yöneticisi örneği oluşturur.
    /// Kernelden gerekli kaynakları (display, touchscreen) edinir.
    fn new(console: &mut ConsoleWriter) -> Result<Self, SahneError> {
        // Çekirdek ekran kaynağını edin (Okuma/Yazma için)
        let display_handle = resource::acquire("display", resource::MODE_READ | resource::MODE_WRITE)?; // "display" kaynak adını varsayalım

        // TODO: Framebuffer bilgilerini (adres, boyut, format) çekirdekten al.
         resource::control(display_handle, GET_FRAMEBUFFER_INFO_CMD, &mut info_struct)?; // Varsayımsal control syscall'ı
        // Veya drivers::display modülünden doğrudan alınabilir eğer öyle taslandıysa.
        let fb_width = 800; // Varsayımsal
        let fb_height = 600; // Varsayımsal
        let fb_pixel_size = 4; // Varsayımsal (32-bit ARGB/RGBA)
        let fb_size = (fb_width * fb_height * fb_pixel_size) as usize;

        // Framebuffer belleğine erişim (resource::control veya başka bir yolla alınmalı)
        // Eğer kernel sadece Handle veriyorsa ve bellek eşlemesi gerekiyorsa sahne64::memory::map_shared kullanılabilir.
        // En basiti: kernel driver'ı bir pointer döner.
        let framebuffer_ptr = unsafe {
             // Geçici: doğrudan display sürücüsünden framebuffer pointer'ı aldığımızı varsayalım
             crate::drivers::display::framebuffer().map(|s| s.as_mut_ptr())
            // Veya resource control ile pointer alımı
            let mut ptr: *mut u8 = ptr::null_mut();
            resource::control(display_handle, GET_FRAMEBUFFER_PTR_CMD, &mut ptr)?;
            // Şimdilik sabit varsayımsal adres
             0xZAAA_0000 as *mut u8 // drivers/display.rs'deki varsayımsal adres
        };
        let framebuffer = ptr::NonNull::new(framebuffer_ptr);


        if framebuffer.is_none() {
            writeln!(console, "Hata: Framebuffer belleği alınamadı.").unwrap();
             return Err(SahneError::ResourceNotFound);
        }


        // Çekirdek dokunmatik ekran kaynağını edin (Okuma için)
        let touchscreen_handle = resource::acquire("touchscreen", resource::MODE_READ)?; // "touchscreen" kaynak adını varsayalım


        // TODO: IPC sunucu endpoint'ini başlat
         let server_endpoint = ipc::listen("display_server")?; // Varsayımsal IPC dinleyici


        writeln!(console, "SahneWM başlatıldı. Çözünürlük {}x{}, {} BPP.", fb_width, fb_height, fb_pixel_size * 8).unwrap();

        Ok(SahneWindowManager {
            display_handle,
            touchscreen_handle,
            framebuffer,
            fb_width, fb_height, fb_pixel_size, fb_size,
            windows: Vec::new(),
            next_window_id: 1, // ID 0 geçersiz kabul edilebilir
             server_endpoint,
        })
    }

    // Yeni bir pencere oluşturur (Uygulamadan gelen IPC isteği üzerine)
    // TODO: IPC isteği detayları (boyut, konum vb.) parametre olarak gelmeli
    fn create_window(&mut self, width: u32, height: u32) -> Result<u32, SahneError> {
        // Pencere ID'si ata
        let window_id = self.next_window_id;
        self.next_window_id += 1;

        // TODO: Pencere içeriği için paylaşımlı bellek alanı oluştur/tahsis et.
        // Sahne64::memory::create_shared kullanılır ve Handle'ı istemciye gönderilir.
        // Sunucu da bu Handle'ı kendi adres alanına eşler (memory::map_shared).
        let buffer_size = (width * height * self.fb_pixel_size) as usize;
         let shared_mem_handle = memory::create_shared(buffer_size)?; // Paylaşımlı bellek handle'ı
         let content_buffer_ptr = memory::map_shared(shared_mem_handle, 0, buffer_size)?; // Kendi adres alanımıza eşle

        let content_buffer_ptr: *mut u8 = memory::allocate(buffer_size)?; // Basitlik için doğrudan heap'ten tahsis (Paylaşımlı bellek daha verimli)

        if content_buffer_ptr.is_null() {
             memory::release(shared_mem_handle)?; // Paylaşımlı bellek oluşturulduysa serbest bırak
             return Err(SahneError::OutOfMemory);
        }
        // Belleği sıfırla (örneğin siyaha boya)
         unsafe { ptr::write_bytes(content_buffer_ptr, 0, buffer_size); }


        // Yeni Window yapısı oluştur
        let new_window = Window {
            id: window_id,
            x: 50 + (self.windows.len() * 20) as i32, // Basit yerleştirme
            y: 50 + (self.windows.len() * 20) as i32,
            width, height,
            content_buffer: ptr::NonNull::new(content_buffer_ptr),
            buffer_size,
             client_endpoint: Some(client_connection), // İstemci bağlantısını kaydet
            z_order: self.windows.len() as u32, // Basit z-sırası (listede sonuncusu üstte)
        };

        self.windows.push(new_window);

        // TODO: İstemciye başarı mesajı ve pencere ID'si/paylaşımlı bellek Handle'ı gönder.
         ipc::send_message(client_connection, WindowCreatedMessage { id: window_id, shared_mem_handle, ... });

        Ok(window_id)
    }

    // Pencereyi kapatır (Uygulamadan gelen IPC isteği üzerine)
    fn close_window(&mut self, window_id: u32) -> Result<(), SahneError> {
        // Pencereyi listeden bul
        let index = self.windows.iter().position(|w| w.id == window_id);

        if let Some(index) = index {
            let mut window = self.windows.remove(index);
            // TODO: İstemci bağlantısını kapat (ipc::close_connection)
            // TODO: Pencere içeriği için ayrılan belleği serbest bırak.
            if let Some(ptr) = window.content_buffer.take() {
                 // Eğer paylaşımlı bellek kullanıldıysa memory::unmap_shared ve memory::release kullanılmalı
                 // Basit heap tahsisi kullanıldıysa memory::release kullanılır.
                 memory::release(ptr.as_ptr(), window.buffer_size)?; // memory::release(ptr, size)
            }

            // TODO: Z-sırasını güncelle (çıkarılan pencereden sonrakilerin z-sırasını azalt)

            Ok(())
        } else {
            Err(SahneError::InvalidHandle) // Pencere ID'si bulunamadı
        }
    }

    // Tüm pencereleri ana frambuffer üzerine birleştirerek çizer.
    // TODO: Bu fonksiyon periyodik olarak veya bir pencerenin içeriği değiştiğinde çağrılmalıdır.
    fn composite_and_draw(&mut self) {
        let Some(framebuffer_ptr) = self.framebuffer else { return; };
        let framebuffer = unsafe { slice::from_raw_parts_mut(framebuffer_ptr.as_ptr(), self.fb_size) };

        // Ana framebuffer'ı temizle (örneğin siyah veya arka plan rengi)
        unsafe { ptr::write_bytes(framebuffer_ptr.as_ptr(), 0x10, self.fb_size); } // Koyu gri (varsayımsal)


        // Pencereleri Z-sırasına göre sırala (en alttaki ilk çizilir)
        // Basitlik için listeyi kopyalayıp sıralayalım
        let mut sorted_windows = self.windows.clone(); // Window'un Clone traitini implemente etmesi gerek
        sorted_windows.sort_by_key(|w| w.z_order);


        // Pencereleri birleştirerek çiz
        for window in &mut sorted_windows {
            window.composite_to_framebuffer(framebuffer, self.fb_width, self.fb_height, self.fb_pixel_size);
        }

        // TODO: Eğer double buffering kullanılıyorsa, buraya swap_buffers syscall'ı çağrılır.
         resource::control(self.display_handle, SWAP_BUFFERS_CMD, ptr::null())?; // Varsayımsal control syscall'ı
    }

    // Girdi olaylarını işler ve doğru pencereye yönlendirir.
    // TODO: touchscreen::poll_event API'si resource::read üzerine kurulmuşsa kullanılır.
    fn handle_input_events(&mut self) {
        // TODO: Dokunmatik ekran kaynağından olayları oku (polling veya kesme).
         resource::read(self.touchscreen_handle, &mut event_buffer, ...)?;

        // Örnek dokunma olayı işleme:
         let event = touchscreen::poll_event(); // Doğrudan sürücüye erişim varsayımı

        // Eğer resource olarak sunuluyorsa:
         let mut touch_event_buffer = [0u8; 8]; // Varsayımsal olay boyutu
         match resource::read(self.touchscreen_handle, &mut touch_event_buffer, 0, touch_event_buffer.len()) {
             Ok(bytes_read) if bytes_read > 0 => {
                 // TODO: Okunan baytları TouchEvent yapısına çevir.
                  let touch_event: TouchEvent = parse_touch_event(&touch_event_buffer[0..bytes_read]);

                 // Varsayımsal TouchEvent
                 struct TouchEvent { kind: u8, x: u16, y: u16 }
                 const TOUCH_KIND_PRESSED: u8 = 1;
                 let touch_event = unsafe {
                      let ptr = touch_event_buffer.as_ptr() as *const TouchEvent;
                      ptr::read_unaligned(ptr) // Paketlenmiş yapı olabilir
                 };


                 // Olayın hangi pencerede olduğunu bul (en üstteki pencereden başlayarak)
                 // Pencereleri z-sırasına göre tersten sırala (en üstteki ilk)
                 let mut windows_sorted_by_z = self.windows.iter_mut().collect::<Vec<_>>();
                 windows_sorted_by_z.sort_by_key(|w| core::cmp::Reverse(w.z_order));

                 for window in windows_sorted_by_z {
                     if window.contains_point(touch_event.x as i32, touch_event.y as i32) {
                         // Olay bu pencerede, istemci uygulamaya IPC ile olayı gönder.
                         ipc::send_message(window.client_endpoint, InputEventMessage { event_type, x, y, ... });
                          printk!("DEBUG: Dokunma olayı pencere {}'ye yönlendirildi.\n", window.id);

                         // Eğer basma olayıysa, bu pencereyi en üste getir (z-sırasını güncelle)
                         if touch_event.kind == TOUCH_KIND_PRESSED {
                              self.bring_window_to_front(window.id);
                         }
                         break; // Olayı tek bir pencereye yönlendir
                     }
                 }

             }
             _ => {} // Veri yok veya hata
         }
    }

    // Bir pencereyi Z-sırasında en üste getirir.
    fn bring_window_to_front(&mut self, window_id: u32) {
        let Some(index) = self.windows.iter().position(|w| w.id == window_id) else { return; };

        // Seçilen pencerenin z-sırasını en yükseğe ayarla
        let current_max_z = self.windows.iter().map(|w| w.z_order).max().unwrap_or(0);
        let old_z = self.windows[index].z_order;
        self.windows[index].z_order = current_max_z + 1;

        // Diğer pencerelerin z-sırasını güncellemeye gerek yok (en basiti)
        // Veya daha temiz bir yöntem: en üste gelen hariç diğerlerinin z-sırasını düşür
         for win in &mut self.windows {
             if win.id != window_id && win.z_order > old_z {
                 win.z_order -= 1;
             }
         }
        // Sonra listeyi yeniden sıralayabiliriz (composite_and_draw içinde yapılıyor).
         // printk!("DEBUG: Pencere {} en öne getirildi.\n", window_id);
    }

    // Ana çalışma döngüsü
    fn run(&mut self) -> ! {
        // TODO: IPC sunucu endpoint'ini dinlemeye başla (ipc::listen çağrısı new içinde yapıldı)
        // Ana döngü: Girdi olaylarını işle, IPC isteklerini işle, ekranı güncelle.
        loop {
            // 1. Girdi Olaylarını İşle (Dokunma vb.)
            self.handle_input_events();

            // 2. IPC İsteklerini İşle (Pencere oluşturma, çizim komutları, olay onayları vb.)
            // Bu kısım IPC mekanizmasına bağlıdır.
            // Örneğin, gelen bağlantıları kabul et (ipc::accept)
            // Gelen mesajları oku (ipc::receive_message)
            // Mesaj tipine göre ilgili fonksiyonları çağır (create_window, close_window, draw_command, etc.)
             if let Some(connection) = self.server_endpoint.accept() { // Varsayımsal kabul
                // Yeni istemci bağlantısı işleme
             }
             if let Some(message) = ipc::receive_message() { // Varsayımsal mesaj alma
                match message {
                    CreateWindowMessage { width, height, ... } => { self.create_window(width, height).unwrap(); },
                    CloseWindowMessage { id } => { self.close_window(id).unwrap(); },
                    DrawRectMessage { id, x, y, w, h, color } => { /* Pencereyi bul ve çizim yap */ },
            //        // ... diğer mesaj tipleri
                }
             }


            // 3. Ekranı Güncelle (Pencereleri birleştirerek çiz)
            // Sadece bir şeyler değiştiğinde veya periyodik olarak çizmek daha verimlidir.
            // Basitlik için her döngüde çizelim:
             self.composite_and_draw();

            // TODO: Eğer hiçbir şey olmadıysa işlemciyi serbest bırakmak önemlidir.
             task::yield_now().unwrap_or_else(|_| { core::hint::spin_loop(); });
             // Eğer polling yoksa, IPC'den veya girdi kesmesinden uyanmayı bekle.
        }

        // Sunucu asla dönmez
         task::exit(0); // Normalde buraya gelinmez
    }
}


// Pencereleme Sunucusu Uygulamasının Ana Giriş Noktası
#[no_mangle]
pub extern "C" fn main(_argc: usize, _argv: *const *const u8) -> ! {
    // Konsol kaynağını edin (Hata ayıklama için)
    let console_handle = resource::acquire("console", resource::MODE_WRITE).unwrap_or_else(|_| { loop { core::hint::spin_loop(); } });
    let mut console_writer = ConsoleWriter { handle: console_handle };

    writeln!(console_writer, "SahneBox Pencereleme Sunucusu (sahnewm) Başlıyor.").unwrap();

    // Pencere Yöneticisini Başlat
    match SahneWindowManager::new(&mut console_writer) {
        Ok(mut wm) => {
            // Ana döngüyü çalıştır
            wm.run();
        }
        Err(err) => {
             writeln!(console_writer, "Hata: Pencere Yöneticisi başlatılamadı: {:?}", err).unwrap();
             // Başlatma hatası, sistem grafik arayüzü olmadan çalışacak
             // veya sadece konsol modunda kalacak.
             // Bu görev hata koduyla çıksın veya sonsuz döngüye girsin.
             task::exit(-1);
        }
    }

    // Buraya asla ulaşılmamalıdır.
    loop {}
}


// --- Kullanıcı Alanı Grafik/Pencereleme Kütüphanesi İskeleti ---
// windows_system/libsaheneui_minimal/src/lib.rs
// Uygulamalar bu kütüphaneye bağlanacak.

 #![no_std]
 #![feature(alloc)]
 extern crate alloc;
 use alloc::string::String;
 use alloc::vec::Vec;
 use alloc::boxed::Box;
 use crate::sahne64::{self, ipc, SahneError, Handle}; // ipc modülü varsayımı

// TODO: Pencere Oluşturma, Çizim, Olay İşleme API'leri burada tanımlanacak.
 struct DisplayConnection { server_endpoint: ipc::Connection? }
 struct Window { id: u32, connection: DisplayConnection?, shared_mem_buffer: Option<&'static mut [u8]> } // Paylaşımlı bellek varsa

// // Sunucuya bağlan
 pub fn connect_to_display_server() -> Result<DisplayConnection, SahneError> {
      ipc::connect("display_server")? // Varsayımsal IPC bağlantısı
       Ok(DisplayConnection { server_endpoint: Some(connection) })
     Err(SahneError::NotSupported) // Henüz implemente edilmedi
 }

// // Pencere oluştur (IPC ile sunucuya istek gönderir)
 pub fn create_window(connection: &mut DisplayConnection, width: u32, height: u32, title: &str) -> Result<Window, SahneError> {
//     // IPC mesajı gönder: CreateWindow { width, height, title }
      Sunucudan yanıt bekle: WindowCreated { id, shared_mem_handle, ... }
     // Shared mem handle'ı kendi adres alanına eşle: memory::map_shared
     Err(SahneError::NotSupported) // Henüz implemente edilmedi
 }

// // Pencereye çizim yapma (örn. doğrudan paylaşımlı belleğe yazma)
 impl Window { pub fn get_buffer(&mut self) -> Option<&mut [u8]> { self.shared_mem_buffer.as_deref_mut() } }

// // Girdi olaylarını al (IPC ile sunucudan mesaj al)
 impl Window { pub fn get_event(&mut self) -> Option<InputEvent> { ... } }
 pub enum InputEvent { Touch { x: u16, y: u16, kind: TouchEventKind }, ... } // touchscreen.rs'deki enum kullanılabilir

// // Pencereyi kapat (IPC ile sunucuya istek gönderir)
 impl Window { pub fn close(self) -> Result<(), SahneError> { ... } }