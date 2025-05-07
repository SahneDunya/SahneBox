# main_kernel/asm.S
# Genel Amaçlı RISC-V 64-bit (RV64) Assembly Fonksiyonları

.section .text.entry # Çekirdek metin bölümüne yerleştir
.global disable_interrupts # Rust'tan çağrılabilir yap
.global enable_interrupts  # Rust'tan çağrılabilir yap
.global halt_cpu           # Rust'tan çağrılabilir yap
.global read_csr           # Rust'tan çağrılabilir yap
.global write_csr          # Rust'tan çağrılabilir yap

# void disable_interrupts(void);
# Makine kesmelerini (MIE biti) devre dışı bırakır.
# RV64 makine modunda MIE bitini kontrol etmek için mstatus CSR kullanılır.
disable_interrupts:
    csrr a0, mstatus      # mstatus CSR değerini a0'a oku
    andi a0, a0, ~8       # MIE bitini (bit 3) sıfırla (maske ~8 veya 0xFFFFFFF7)
    csrw mstatus, a0      # Güncellenmiş değeri mstatus'a yaz
    ret                   # Fonksiyondan dön

# void enable_interrupts(void);
# Makine kesmelerini (MIE biti) etkinleştirir.
enable_interrupts:
    csrr a0, mstatus      # mstatus CSR değerini a0'a oku
    ori a0, a0, 8         # MIE bitini (bit 3) set et (maske 8 veya 0x8)
    csrw mstatus, a0      # Güncellenmiş değeri mstatus'a yaz
    ret                   # Fonksiyondan dön

# __attribute__((noreturn)) void halt_cpu(void);
# İşlemciyi bekletir veya durdurur.
# Basitçe bir "wait for interrupt" (wfi) döngüsü.
halt_cpu:
    wfi                   # Bir kesme olana kadar bekle
    j halt_cpu            # Kesme gelirse tekrar bekle (sonsuz döngü gibi davranır)
    # veya basitçe:
    # 1: b 1b            # Sonsuz döngü (eğer wfi kullanılmak istenmiyorsa)
    # ret               # Bu fonksiyondan normalde dönülmez

# unsigned long read_csr(unsigned long csr_address);
# Belirtilen CSR adresindeki değeri okur.
# csr_address a0 register'ında gelir.
# Okunan değer a0 register'ında döner.
read_csr:
    csrr a0, a0           # a0'daki CSR adresinden oku, sonucu a0'a yaz
    ret                   # Fonksiyondan dön

# void write_csr(unsigned long csr_address, unsigned long value);
# Belirtilen CSR adresine değeri yazar.
# csr_address a0 register'ında gelir.
# value a1 register'ında gelir.
write_csr:
    csrw a0, a1           # a0'daki CSR adresine a1'deki değeri yaz
    ret                   # Fonksiyondan dön

# Diğer genel yardımcı fonksiyonlar buraya eklenebilir
# Örneğin, bellek bariyerleri (fences) veya atomik operasyonlar için.
# Ancak rs_io.S dosyası I/O odaklı bariyerleri içerecek.