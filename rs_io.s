# main_kernel/rs_io.S
# Rust I/O için RISC-V 64-bit (RV64) Assembly Yardımcıları

.section .text          # Çekirdek metin bölümüne yerleştir
.global mmio_read32     # Rust'tan çağrılabilir yap
.global mmio_write32    # Rust'tan çağrılabilir yap
.global mmio_read64     # Rust'tan çağrılabilir yap
.global mmio_write64    # Rust'tan çağrılabilir yap
.global io_fence_r_rw   # Rust'tan çağrılabilir yap
.global io_fence_w_rw   # Rust'tan çağrılabilir yap

# unsigned int mmio_read32(unsigned long addr);
# Belirtilen 32-bit bellek adresinden (MMIO) değer okur. Volatile erişim için kullanılır.
# addr a0 register'ında gelir.
# Okunan değer a0 register'ında döner.
mmio_read32:
    lwu a0, 0(a0)         # Adresten 32-bit işaretsiz kelime oku (Zero-extend to 64-bit)
    # Genellikle MMIO okumalarından sonra bir okuma bariyeri önerilir.
    # fence r, rw         # Okuma sonrası okuma/yazma bariyeri
    ret                   # Fonksiyondan dön

# void mmio_write32(unsigned long addr, unsigned int val);
# Belirtilen 32-bit bellek adresine (MMIO) değer yazar. Volatile erişim için kullanılır.
# addr a0 register'ında gelir.
# val a1 register'ında gelir.
mmio_write32:
    sw a1, 0(a0)          # Adrese 32-bit kelime yaz
    # Genellikle MMIO yazımlarından sonra bir yazma bariyeri önerilir.
    fence w, rw           # Yazma sonrası okuma/yazma bariyeri
    ret                   # Fonksiyondan dön

# unsigned long mmio_read64(unsigned long addr);
# Belirtilen 64-bit bellek adresinden (MMIO) değer okur. Volatile erişim için kullanılır.
# addr a0 register'ında gelir.
# Okunan değer a0 register'ında döner.
mmio_read64:
    ld a0, 0(a0)          # Adresten 64-bit çift kelime oku
    # Genellikle MMIO okumalarından sonra bir okuma bariyeri önerilir.
    # fence r, rw         # Okuma sonrası okuma/yazma bariyeri
    ret                   # Fonksiyondan dön

# void mmio_write64(unsigned long addr, unsigned long val);
# Belirtilen 64-bit bellek adresine (MMIO) değer yazar. Volatile erişim için kullanılır.
# addr a0 register'ında gelir.
# val a1 register'ında gelir.
mmio_write64:
    sd a1, 0(a0)          # Adrese 64-bit çift kelime yaz
    # Genellikle MMIO yazımlarından sonra bir yazma bariyeri önerilir.
    fence w, rw           # Yazma sonrası okuma/yazma bariyeri
    ret                   # Fonksiyondan dön

# void io_fence_r_rw(void);
# Bir "fence r, rw" yönergesi yayınlar. Önceki okumaların sonraki okuma/yazmalardan önce tamamlanmasını sağlar.
io_fence_r_rw:
    fence r, rw
    ret

# void io_fence_w_rw(void);
# Bir "fence w, rw" yönergesi yayınlar. Önceki yazımların sonraki okuma/yazmalardan önce tamamlanmasını sağlar.
io_fence_w_rw:
    fence w, rw
    ret

# Diğer G/Ç ile ilgili yardımcı fonksiyonlar buraya eklenebilir.
# Örneğin, tam bellek bariyeri (fence rw, rw), instruction fence (fence.i) vb.