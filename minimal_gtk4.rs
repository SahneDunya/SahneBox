// minimal_gtk4/src/lib.rs
// Minimal SahneBox Grafiksel Kullanıcı Arayüzü Araç Seti (GTK Benzeri)

#![no_std]
#![feature(alloc)]
#![feature(box_into_inner)] // Box::into_inner kullanmak için

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::fmt::Write;
use core::slice;
use core::ptr;
use core::cell::RefCell; // Widget'ların iç durumunu değiştirmek için (render metodu &self alırsa)
use core::any::Any; // Widget trait'ini downcast etmek için (karmaşıklığı artırır)


// SahneBox Çekirdek API'si
use crate::sahne64::{self, memory, task, SahneError};

// Minimal Pencereleme Sistemi Kütüphanesi
use crate::windows_system::libsaheneui_minimal::{self, DisplayConnection, Window as SahneWindow, InputEvent, TouchEventKind};


// TODO: Temel Çizim Yardımcıları (Framebuffer üzerine çizim için)
// Doğrudan paylaşımlı pencere tamponuna çizim yapacaklar.
// Renk temsili (örn. u32 ARGB veya u16 RGB565) display driver ve pencereleme sistemi ile uyumlu olmalı.
struct Painter {
    buffer: ptr::NonNull<u8>, // Çizilecek tampon
    width: u32, height: u32, // Tampon boyutu
    pixel_size: u32, // Piksel başına bayt sayısı
}

impl Painter {
    fn new(buffer: ptr::NonNull<u8>, width: u32, height: u32, pixel_size: u32) -> Self {
         Painter { buffer, width, height, pixel_size }
    }

    // Belirtilen koordinata piksel çizer (bounds check dahil)
    fn draw_pixel(&mut self, x: i32, y: i32, color: u32) { // Renk u32 ARGB varsayımı
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let index = ((y as u32 * self.width + x as u32) * self.pixel_size) as usize;
            let buffer_slice = unsafe { slice::from_raw_parts_mut(self.buffer.as_ptr().add(index), self.pixel_size as usize) };

            // TODO: Renk formatına göre baytları yaz (u32 ARGB -> buffer formatı)
            // Örneğin 32-bit ARGB için:
            buffer_slice.copy_from_slice(&color.to_le_bytes());
            // Örneğin 16-bit RGB565 için: color u32 -> u16 dönüşümü ve sonra to_le_bytes()
        }
    }

    // Dikdörtgen çizer (bounds check dahil)
    fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: u32) {
        for dy in 0..h {
            for dx in 0..w {
                self.draw_pixel(x + dx as i32, y + dy as i32, color);
            }
        }
    }

    // TODO: Metin çizme (Font yükleme ve rendering gerektirir - ÇOK KARMAŞIK)
    fn draw_text(&mut self, x: i32, y: i32, text: &str, color: u32, font: &Font) { ... }
}


// Widget trait'i: Tüm UI bileşenleri için ortak arayüz
// Clone, Debug gibi traitler gerekebilir.
pub trait Widget {
    fn draw(&self, painter: &mut Painter, offset_x: i32, offset_y: i32); // Widget'ı çizer (kendi pozisyonuna göre offset eklenir)
    // Olay işleyici (kendi koordinatlarına göre x, y)
    // Dönüş değeri: Olay işlendi mi, yeniden çizim gerekli mi?
    fn handle_event(&mut self, event: &InputEvent, event_x: i32, event_y: i32) -> bool;
    fn get_preferred_size(&self) -> (u32, u32); // Widget'ın tercih ettiği boyut (genişlik, yükseklik)
    fn set_position(&mut self, x: i32, y: i32); // Widget'ın pozisyonunu ayarlar
    fn get_position(&self) -> (i32, i32); // Widget'ın pozisyonunu döndürür

    // TODO: get_size, set_size fonksiyonları
    // TODO: set_parent, get_parent fonksiyonları
    // TODO: Container trait (children yönetimi için)
}


// Temel Widget Alanları (Çok fazla kopyalamayı önlemek için struct yerine trait ve getter/setter kullanılabilir)
// Ama basitlik için her widget kendi alanlarını tutabilir veya ortak bir BaseWidget struct kullanılabilir.
// Basitlik için şimdilik her widget kendi alanlarını tutsun ve Widget trait'i getter/setterları tanımlasın.


// Label Widget'ı
pub struct Label {
    text: String,
    x: i32, y: i32, // Kendi pozisyonu
    width: u32, height: u32, // Kendi boyutu (tercih edilen boyut, layout tarafından ayarlanır)
    // TODO: Renk, Font gibi özellikler
}

impl Label {
    pub fn new(text: &str) -> Box<Self> {
        Box::new(Label {
            text: text.to_string(),
            x: 0, y: 0, width: 0, height: 0,
        })
    }
}

impl Widget for Label {
    fn draw(&self, painter: &mut Painter, offset_x: i32, offset_y: i32) {
        let draw_x = self.x + offset_x;
        let draw_y = self.y + offset_y;
        // TODO: Metni çizme (Painter::draw_text gerektirir - şimdilik sadece arka planı çizelim)
         painter.draw_rect(draw_x, draw_y, self.width, self.height, 0xFF808080); // Gri arka plan
          printk!("DEBUG: Label çizildi: {} @ ({},{}) size ({},{})\n", self.text, draw_x, draw_y, self.width, self.height);
         // Gerçekte burada metin çizimi olmalı
    }

    fn handle_event(&mut self, event: &InputEvent, event_x: i32, event_y: i32) -> bool {
        // Label'lar genellikle girdi olaylarını işlemez
        false // Olay işlenmedi
    }

    fn get_preferred_size(&self) -> (u32, u32) {
        // TODO: Metnin boyutunu hesaplama (Font metrikleri ve text rendering gerektirir - ÇOK ZOR)
        // Şimdilik sabit bir boyut dönelim
        let char_width = 6; // Varsayımsal karakter genişliği
        let char_height = 12; // Varsayımsal karakter yüksekliği
        let width = (self.text.len() as u32) * char_width;
        let height = char_height;
        (width + 4, height + 4) // Biraz padding ekle
    }

    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn get_position(&self) -> (i32, i32) { (self.x, self.y) }
}


// Button Widget'ı
pub struct Button {
    label: Box<Label>, // Butonun üzerindeki etiket
    x: i32, y: i32,
    width: u32, height: u32,
    is_pressed: bool, // Durum
    // TODO: Tıklama sinyali/callback
    on_click: Option<Box<dyn Fn() + 'static>>, // Tıklama olayı için callback
}

impl Button {
    pub fn new(text: &str) -> Box<Self> {
        Box::new(Button {
            label: Label::new(text),
            x: 0, y: 0, width: 0, height: 0,
            is_pressed: false,
            on_click: None,
        })
    }

    // Tıklama callback'ini ayarlar
    pub fn connect_clicked<F>(&mut self, callback: F) where F: Fn() + 'static {
        self.on_click = Some(Box::new(callback));
    }
}

impl Widget for Button {
    fn draw(&self, painter: &mut Painter, offset_x: i32, offset_y: i32) {
        let draw_x = self.x + offset_x;
        let draw_y = self.y + offset_y;
        let bg_color = if self.is_pressed { 0xFF4040FF } else { 0xFF8080FF }; // Basılıysa koyu mavi, değilse açık mavi

        painter.draw_rect(draw_x, draw_y, self.width, self.height, bg_color);

        // Etiketi çiz (merkezlemeye dikkat et)
        let (label_w, label_h) = self.label.get_preferred_size();
        let label_draw_x = draw_x + (self.width as i32 - label_w as i32) / 2;
        let label_draw_y = draw_y + (self.height as i32 - label_h as i32) / 2;
        self.label.set_position(label_draw_x - draw_x, label_draw_y - draw_y); // Label pozisyonu parent'a göre
        self.label.draw(painter, offset_x, offset_y); // Label'ı çiz
         printk!("DEBUG: Button çizildi: @ ({},{}) size ({},{})\n", draw_x, draw_y, self.width, self.height);
    }

    fn handle_event(&mut self, event: &InputEvent, event_x: i32, event_y: i32) -> bool {
        let handled = false;
        // Olay koordinatları widget'ın içinde mi?
        if event_x >= self.x && event_x < (self.x + self.width as i32) &&
           event_y >= self.y && event_y < (self.y + self.height as i32)
        {
            match event {
                InputEvent::Touch { kind, x, y } => {
                    match kind {
                        TouchEventKind::Pressed => {
                            self.is_pressed = true;
                             printk!("DEBUG: Button basıldı @ ({},{})", event_x, event_y);
                             return true; // Olay işlendi, yeniden çizim gerek
                        }
                        TouchEventKind::Released => {
                            if self.is_pressed {
                                self.is_pressed = false;
                                // Tıklama gerçekleşti, callback'i çağır
                                 printk!("DEBUG: Button bırakıldı @ ({},{})", event_x, event_y);
                                if let Some(callback) = &self.on_click {
                                    // Callback'i çağırmak unsafe veya dikkatli thread yönetimi gerektirebilir
                                     callback(); // Callback çalıştırma
                                      printk!("DEBUG: Button callback çağrıldı.");
                                }
                                return true; // Olay işlendi, yeniden çizim gerek
                            }
                        }
                        TouchEventKind::Moved => {
                             // Basılıyken dışarı sürüklenirse is_pressed false yapılabilir
                             if self.is_pressed && !self.contains_point(*x as i32, *y as i32) {
                                 self.is_pressed = false;
                                 return true; // Durum değişti, yeniden çizim gerek
                             } else if !self.is_pressed && self.contains_point(*x as i32, *y as i32) {
                                 // Dışarıdayken içeri sürüklenirse is_pressed true yapılabilir
                                 self.is_pressed = true;
                                 return true; // Durum değişti, yeniden çizim gerek
                             }
                        }
                    }
                }
                // TODO: Klavye, Fare olayları
            }
        } else {
             // Olay widget dışında, eğer basılıyken dışarı çıkıldıysa basılı durumunu temizle
             if self.is_pressed {
                 self.is_pressed = false;
                 return true; // Durum değişti, yeniden çizim gerek
             }
        }
        handled // Olay işlenmedi
    }

    fn get_preferred_size(&self) -> (u32, u32) {
        let (label_w, label_h) = self.label.get_preferred_size();
        // Etiketin boyutundan biraz daha büyük bir buton boyutu
        (label_w + 10, label_h + 10)
    }

    fn set_position(&mut self, x: i32, y: i32) { self.x = x; self.y = y; }
    fn get_position(&self) -> (i32, i32) { (self.x, self.y) }
}


// Dikey Kutucuk (VBox) Container Widget'ı
pub struct VBox {
    children: Vec<Box<dyn Widget>>, // Çocuk widget'lar
    x: i32, y: i32,
    width: u32, height: u32, // Layout tarafından ayarlanır
    padding: u32, // Çocuklar arasındaki boşluk
    // TODO: Hizalama (alignment)
}

impl VBox {
    pub fn new(padding: u32) -> Box<Self> {
        Box::new(VBox {
            children: Vec::new(),
            x: 0, y: 0, width: 0, height: 0,
            padding,
        })
    }

    pub fn add(&mut self, widget: Box<dyn Widget>) {
        self.children.push(widget);
    }

    // Çocukları dikey olarak düzenler ve kendi boyutunu hesaplar.
    fn perform_layout(&mut self) {
        let mut current_y = self.y;
        let mut max_width = 0;
        let mut total_height = 0;

        for child in &mut self.children {
            let (pref_w, pref_h) = child.get_preferred_size();
            max_width = core::cmp::max(max_width, pref_w);
            // Çocuğun pozisyonunu ayarla
            child.set_position(self.x, current_y); // VBox'ın kendi pozisyonuna göre
            current_y += pref_h as i32 + self.padding as i32;
            total_height += pref_h + self.padding;
        }

        if total_height > 0 { total_height -= self.padding; } // Son elemandan sonra padding olmamalı

        // VBox'ın boyutunu ayarla
        self.width = max_width; // En geniş çocuğun genişliği
        self.height = total_height;
    }
}

impl Widget for VBox {
     fn draw(&self, painter: &mut Painter, offset_x: i32, offset_y: i32) {
        // VBox kendisi görünmez olabilir veya bir arka plan çizebilir
         paint.draw_rect(self.x + offset_x, self.y + offset_y, self.width, self.height, 0xFF303030); // Koyu gri arka plan (isteğe bağlı)

        // Çocukları çiz
        for child in &self.children {
            child.draw(painter, offset_x, offset_y); // Çocuk kendi pozisyonunda çizilir
        }
     }

    fn handle_event(&mut self, event: &InputEvent, event_x: i32, event_y: i32) -> bool {
        let mut handled = false;
        // Olay koordinatları bu container'ın içinde mi? (Opsiyonel, tüm olayları çocuklara gönderebilir)
        // Olay koordinatlarını çocuğun koordinat sistemine göre dönüştür
        let child_event_x = event_x - self.x;
        let child_event_y = event_y - self.y;

        // Olayı çocuk widget'lara yönlendir (tersten, en üstteki çocuğa önce)
        for child in self.children.iter_mut().rev() { // rev() z-sırası yönetimi için önemli olabilir
            let (child_x, child_y) = child.get_position();
            // Olay bu çocuğun bounds'u içinde mi?
            let (child_w, child_h) = child.get_preferred_size(); // Veya layout tarafından belirlenen boyut
             if child_event_x >= child_x && child_event_x < (child_x + child_w as i32) &&
                child_event_y >= child_y && child_event_y < (child_y + child_h as i32)
             {
                if child.handle_event(event, child_event_x, child_event_y) {
                    handled = true; // Olay işlendi
                    // Eğer çocuk işlediyse, olayı başka çocuklara göndermeyi durdurabiliriz (genellikle böyledir)
                    break;
                }
             }
        }
        handled // Olay işlendi mi?
    }

    fn get_preferred_size(&self) -> (u32, u32) {
        // Çocukları düzenlemeden boyut hesaplamak zor.
        // Basitlik için burada da layout yapabilir veya cached boyutu dönebilir.
        // Performans_layout'u çağırmak mutable borrow sorununa yol açabilir.
        // Şimdilik 0 dönelim, VBox boyutu ebeveyni tarafından ayarlanır.
        // Daha iyi bir yöntem: calculate_preferred_size() gibi ayrı bir fonksiyon.
         // Geçici: Layout yapıp boyutu dönelim
        let mut temp_vbox = VBox::new(self.padding);
        for child in self.children.iter() {
             // Çocukları clone veya Box<dyn Widget> tekrar Box<dyn Widget> yapmak zor.
             // Widget trait'ine clone eklemek her widget'ın clone edilebilir olmasını gerektirir.
             // Daha iyi bir yöntem: Layout hesaplamasını ayrı bir fonksiyonda yapmak.
        }
        // Şimdilik tahmini/sabit boyut dönelim
        (200, 300) // Varsayımsal boyut
    }

     fn set_position(&mut self, x: i32, y: i32) {
         let dx = x - self.x;
         let dy = y - self.y;
         self.x = x; self.y = y;
         // Çocukların pozisyonlarını da güncelle
         for child in &mut self.children {
             let (child_x, child_y) = child.get_position();
             child.set_position(child_x + dx, child_y + dy);
         }
     }
     fn get_position(&self) -> (i32, i32) { (self.x, self.y) }

     // TODO: add fonksiyonu Container trait'inde olmalıydı.
}


// Ana Uygulama Yapısı ve Olay Döngüsü
pub struct Application {
    // TODO: Pencereleme sunucusu bağlantısı
    display_connection: Option<DisplayConnection>,
    // TODO: Ana uygulama penceresi
    main_window: Option<SahneWindow>,
    // TODO: Uygulamanın widget ağacının kökü (genellikle bir VBox veya HBox)
    root_widget: Option<Box<dyn Widget>>, // Application penceresinin içeriği

    // TODO: Uygulama durumu, sinyaller vb.
}

impl Application {
    /// Yeni bir uygulama örneği oluşturur.
    pub fn new() -> Result<Self, SahneError> {
        // Pencereleme sunucusuna bağlan
        let display_connection = libsaheneui_minimal::connect_to_display_server().ok(); // Bağlantı başarısız olursa None olur

        if display_connection.is_none() {
              printk!("Hata: Pencereleme sunucusuna bağlanılamadı.\n");
             return Err(SahneError::ResourceNotFound); // Veya IPC hatası
        }

        Ok(Application {
            display_connection,
            main_window: None,
            root_widget: None,
        })
    }

    /// Uygulama penceresini oluşturur ve içeriğini (widget ağacını) belirler.
    /// Widget ağacının kökünü alır.
    pub fn create_main_window(&mut self, width: u32, height: u32, title: &str, root_widget: Box<dyn Widget>) -> Result<(), SahneError> {
        let Some(conn) = &mut self.display_connection else {
             return Err(SahneError::InvalidOperation); // Sunucu bağlantısı yok
        };

        // Pencereyi pencereleme sunucusunda oluştur
        let main_window = conn.create_window(width, height, title)?;
         printk!("DEBUG: Uygulama penceresi oluşturuldu.\n");

        self.main_window = Some(main_window);
        self.root_widget = Some(root_widget);

        // Kök widget'ın boyutunu pencere boyutuna ayarla ve layout yap
        if let Some(root) = &mut self.root_widget {
            root.set_position(0, 0); // Pencerenin sol üst köşesi
            // TODO: Kök widget'ın boyutunu pencere boyutuna ayarlayın ve layout yapın
             root.set_size(width, height);
            if let Some(vbox) = root.as_any_mut().downcast_mut::<VBox>() { // Sadece VBox olduğunu varsayalım (geçici)
                 vbox.width = width; vbox.height = height; // VBox'ın kendi boyutu ayarlanırsa perform_layout içinde çocukları yerleştirir
                 vbox.perform_layout();
            }
        }


        Ok(())
    }


    /// Uygulamanın ana olay döngüsünü çalıştırır.
    pub fn run(&mut self) -> ! {
        let Some(main_window) = &mut self.main_window else {
              printk!("Hata: Ana pencere oluşturulmadı.\n");
             task::exit(-1); // Pencere yoksa uygulama çalışamaz
        };

        // Olay döngüsü
        loop {
            // 1. Pencereleme sunucusundan olayları al
            match main_window.get_event() {
                Some(event) => {
                     printk!("DEBUG: Olay alındı: {:?}\n", event);
                    // 2. Olayı widget hiyerarşisine yönlendir
                    if let Some(root) = &mut self.root_widget {
                         // Olay koordinatlarını kök widget'a göre ayarla (pencereye göre zaten 0,0)
                         let event_x = match event { InputEvent::Touch { x, .. } => *x as i32, /* TODO: Diğer olay tipleri */ _ => 0 };
                         let event_y = match event { InputEvent::Touch { y, .. } => *y as i32, /* TODO: Diğer olay tipleri */ _ => 0 };

                         if root.handle_event(&event, event_x, event_y) {
                            // Olay işlendi ve belki yeniden çizim gerekli
                            // 3. Pencereyi yeniden çiz (basitlik için her olayda)
                            // TODO: Sadece ihtiyaç olduğunda çizimi tetikle
                             printk!("DEBUG: Olay işlendi, yeniden çiziliyor.\n");
                             if let Some(buffer_slice) = main_window.get_buffer_slice() {
                                 let mut painter = Painter::new(ptr::NonNull::new(buffer_slice.as_mut_ptr()).unwrap(), main_window.width, main_window.height, main_window.pixel_size);
                                 root.draw(&mut painter, 0, 0); // Kök widget'ı pencerenin sol üst köşesine çiz
                                 // TODO: Pencere içeriğinin güncellendiğini pencereleme sunucusuna bildir
                                  main_window.notify_content_updated(); // Varsayımsal
                             } else {
                                  printk!("WARN: Pencere buffer'ına erişilemedi.\n");
                             }

                         }
                    }

                }
                None => {
                    // Olay yok, işlemciyi serbest bırak
                    task::yield_now().unwrap_or_else(|_| { core::hint::spin_loop(); }); // Scheduler varsa yield
                     task::sleep(10).unwrap(); // Kısa bir süre uyu (10ms)
                }
            }
        }

        // Olay döngüsünden çıkılmaz
         task::exit(0); // Normalde buraya gelinmez
    }
}

// Widget trait'i Any'den kalıtım almalı veya downcast için başka bir mekanizma olmalı.
// #[feature(downcast-rs)] crate kullanılabilir veya core::any::Any trait'i doğrudan kullanılabilir.
// Basitlik için Widget trait'ine as_any() ve as_any_mut() ekleyelim.
impl dyn Widget {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// Uygulama Giriş Noktası (Örnek Kullanım)
// Ayrı bir user_apps/test_gui/src/main.rs dosyasında olabilir.

#[no_mangle]
pub extern "C" fn main(_argc: usize, _argv: *const *const u8) -> ! {
    // Uygulama oluştur
    let mut app = minimal_gtk4::Application::new().unwrap_or_else(|_| { loop { core::hint::spin_loop(); } }); // Hata durumunda bekle

    // Widget ağacını oluştur
    let mut vbox = minimal_gtk4::VBox::new(5); // 5 piksel padding

    let label = minimal_gtk4::Label::new("Merhaba SahneBox!");
    vbox.add(label);

    let mut button = minimal_gtk4::Button::new("Tıkla Bana");
    // Buton tıklama olayına callback ekle
    button.connect_clicked(|| {
        // TODO: Konsola yazma veya başka bir işlem yapmak için konsol Handle'ı global olabilir
        // veya bir Context struct aracılığıyla callback'e geçirilebilir.
         printk!("Butona Tıklandı!"); // Kernel printk burda çalışmaz
         resource::write(GLOBAL_CONSOLE_HANDLE, b"Butona Tıklandi!\n", 0, ...); // Konsol Handle'ı global/context'ten alınmalı

        // printk! yerini alan kullanıcı alanı çıktısı:
        let console_handle = sahne64::resource::acquire("console", sahne64::resource::MODE_WRITE).unwrap(); // Callback içinde Handle edinmek maliyetli
        let mut writer = ConsoleWriter { handle: console_handle };
        writeln!(writer, "Butona Tıklandı!").unwrap();
        sahne64::resource::release(console_handle).unwrap(); // Handle'ı serbest bırak

    });
    vbox.add(button);


    // Ana pencereyi oluştur
    app.create_main_window(300, 200, "İlk SahneBox Penceresi", vbox).unwrap_or_else(|_| { loop { core::hint::spin_loop(); } });

    // Uygulama olay döngüsünü çalıştır
    app.run(); // Buradan asla dönülmez
}
