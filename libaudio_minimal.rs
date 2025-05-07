// voice_server/libaudio_minimal/src/lib.rs
// Minimal SahneBox Ses Kütüphanesi (Uygulama API'si)

#![no_std] // Standart kütüphane yok
#![feature(alloc)] // Box ve Vec kullanmak için

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::sahne64::{self, resource, SahneError, Handle};

// TODO: Desteklenen Ses Formatı Tanımları
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioFormat {
    pub sample_rate: u32, // Örnek hızı (Hz)
    pub channels: u16,    // Kanal sayısı (1=Mono, 2=Stereo)
    pub bits_per_sample: u16, // Örnek başına bit sayısı (örn. 16)
    // TODO: Format (örneğin S16_LE, U8, vb.) - Enum veya ayrı bir alan olabilir.
}

// Oynatma Akışı
pub struct PlaybackStream {
    audio_handle: Handle, // "audio_out" kaynağına Handle
    format: AudioFormat,
    // TODO: Buffer yönetimi (eğer kütüphane bufferlama yapıyorsa)
}

impl PlaybackStream {
    /// Yeni bir oynatma akışı açar ve belirtilen formatı ayarlamaya çalışır.
    /// format: İstenen ses formatı.
    pub fn open(format: AudioFormat) -> Result<Self, SahneError> {
        // "audio_out" kaynağını edin
        let audio_handle = resource::acquire("audio_out", resource::MODE_WRITE)?; // Oynatma yazma gerektirir

        // TODO: Çekirdek ses kaynağına istenen formatı syscall (resource control?) ile ayarla.
         resource::control(audio_handle, SOME_SET_FORMAT_CMD, &format)?; // Varsayımsal control syscall'ı

        // Başarılı varsayalım
        Ok(PlaybackStream {
            audio_handle,
            format,
        })
    }

    /// Akışa ses verisi yazar (oynatır).
    /// buffer: Oynatılacak ses örneklerini içeren bayt slice'ı.
    pub fn play(&mut self, buffer: &[u8]) -> Result<(), SahneError> {
        if buffer.is_empty() {
            return Ok(());
        }
        // TODO: Buffer'daki veriyi ses kaynağına yaz.
        // resource::write syscall'ı kullanılır.
        // Çekirdek sürücü bu veriyi işleyip donanıma göndermelidir.
        // Yazılan bayt sayısı dönülür, ancak basitlik için tüm buffer'ın yazıldığını varsayalım.
        let bytes_written = resource::write(self.audio_handle, buffer, 0, buffer.len())?; // resource::write(handle, buf, offset, len)
        if bytes_written == buffer.len() {
            Ok(())
        } else {
            // Kısmi yazma veya hata
            Err(SahneError::InvalidOperation) // Daha spesifik hata olabilir
        }
    }

    /// Oynatma akışını kapatır.
    pub fn close(self) -> Result<(), SahneError> {
        // Kaynağı serbest bırak
        resource::release(self.audio_handle)
    }

    // TODO: Akışın durumunu sorgulama (örn. buffer boş mu?)
     pub fn get_status(&self) -> Result<StreamStatus, SahneError>
}

// TODO: Kayıt Akışı (RecordingStream) - open, read, close fonksiyonları ile.

// TODO: Basit bir ses dalgası üretme yardımcı fonksiyonu (test için)
 pub fn generate_sine_wave(frequency: f32, duration_ms: u32, format: &AudioFormat) -> Vec<u8> { ... }