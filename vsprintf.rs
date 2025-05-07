use core::fmt;

// Örnek: Sayıyı çok basitçe stringe çeviren bir yardımcı fonksiyon (genellikle core::fmt::write bunu yapar)
// Sadece konsept göstermek için buradadır, genellikle buna ihtiyacınız olmaz.
#[allow(dead_code)] // Eğer kullanılmazsa uyarı vermemesi için
fn u64_to_string(mut n: u64, buffer: &mut [u8]) -> Option<&str> {
    if buffer.is_empty() {
        return None;
    }
    let mut i = buffer.len() - 1;
    buffer[i] = b'0'; // 0 durumunu ele almak için
    if n == 0 {
         return Some(unsafe { core::str::from_utf8_unchecked(&buffer[i..]) });
    }
    while n > 0 && i > 0 {
        i -= 1;
        buffer[i] = (b'0' + (n % 10) as u8);
        n /= 10;
    }
    if n > 0 { // Buffer yeterince büyük değil
        None
    } else {
        Some(unsafe { core::str::from_utf8_unchecked(&buffer[i..]) })
    }
}