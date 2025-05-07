# mm/page.S
# RISC-V 64-bit (RV64) MMU (Sayfalama) Yardımcı Fonksiyonları

.section .text          # Kod bölümüne yerleştir
.global read_satp       # Rust'tan çağrılabilir yap
.global write_satp      # Rust'tan çağrılabilir yap
.global sfence_vma      # Rust'tan çağrılabilir yap

# unsigned long read_satp(void);
# satp (Supervisor Address Translation and Protection) CSR değerini okur.
# Paging'in durumu ve aktif sayfa tablosunun adresi burada saklanır (Supervisor modunda).
# Makine modunda (M-mode), mstatus'taki MPRV biti ayarlanmadıkça satp etkili değildir.
# Ancak yine de okunabilir/yazılabilir.
read_satp:
    csrr a0, satp         # satp CSR'ı a0 register'ına oku
    ret                   # Fonksiyondan dön

# void write_satp(unsigned long value);
# satp CSR'ına değer yazar.
# value a0 register'ında gelir.
# satp'ye 0 yazmak sayfalama özelliğini devre dışı bırakır.
write_satp:
    csrw satp, a0         # a0'daki değeri satp CSR'ına yaz
    # satp değiştirildikten sonra TLB'nin güncellenmesi gerekir.
    # sfence.vma yönergesi genellikle burada kullanılır.
    sfence.vma zero, zero # Tüm adres alanları ve tüm sanal adresler için SFENCE.VMA
    ret                   # Fonksiyondan dön

# void sfence_vma(void);
# sfence.vma yönergesini çalıştırır.
# Sanal bellek eşlemeleri (TLB) ve/veya data cache girdilerini senkronize etmek için kullanılır.
# Genellikle sayfa tablosu değişikliklerinden sonra veya MMIO erişimleri etrafında gereklidir.
# zero, zero argümanları tüm adres alanları ve tüm sanal adresler için flush yapar.
sfence_vma:
    sfence.vma zero, zero
    ret