# main_kernel/boot/boot.S
# SahneBox Kernel Ana Başlatma Kodu (S-mode)

.section .text          # Kod bölümüne yerleştir
.global _start_rust     # head.S'den çağrılır, global erişilebilir yap
.align 2                # Yönerge hizalaması (4 byte)

.global __stack_top     # Linker scriptten gelecek semboller (extern "C" fn main'e geçirme için)
.global __bss_start
.global __bss_end
.global trap_entry      # traps.rs'deki S-mode trap handler adresi (varsayımsal)

_start_rust:
    # head.S'den gelen argümanlar hala a0 (hartid) ve a1 (dtb_address) registerlarında.

    # 1. Kernel Stack'ini Ayarla
    # Linker scriptten gelen __stack_top sembolünü (stack'in en üst adresi) al ve sp registerına yaz.
    la sp, __stack_top


    # 2. Kernel BSS Bölümünü Sıfırla (.bss = Block Started by Symbol)
    # Linker scriptten gelen __bss_start ve __bss_end sembollerini kullan.
    # __bss_start'tan __bss_end'e kadar olan bellek alanını 0 ile doldur.
    la a0, __bss_start  # a0 = BSS başlangıç adresi
    la a1, __bss_end    # a1 = BSS bitiş adresi
    li a2, 0            # a2 = Sıfır değeri (0)
    # Loop başlat: BSS başlangıcından sonuna kadar 8'er bayt (64 bit) sıfır yaz
clear_bss_loop:
    beq a0, a1, clear_bss_end # Eğer BSS başlangıcı bitişine eşitse (sıfırlama bitti) döngüden çık
    sd a2, (a0)           # Adres a0'a 8 baytlık (sd = store doubleword) sıfır (a2'de 0) yaz
    addi a0, a0, 8        # a0'ı 8 artır (bir sonraki 8 baytlık kelimeye geç)
    j clear_bss_loop      # Döngüyü tekrarla
clear_bss_end:


    # 3. S-Mode Trap Vektörünü Ayarla (stvec)
    # Linker scriptten gelen trap_entry sembolü, kernel'in S-mode trap handler'ının adresidir.
    # stvec CSR'ına bu adresi yaz. Genellikle "Direct" mod kullanılır, bu yüzden adresin son 2 biti 0 olmalıdır (4 bayt hizalama).
    la t0, trap_entry   # t0 = trap_entry adresi
    csrw stvec, t0      # stvec CSR'ına yaz


    # 4. S-Mode CSR'ları Yapılandır (sstatus, sie)
    # sstatus: Supervisor Mode Status Register
    # SIE (Supervisor Interrupt Enable): Kesmeleri etkinleştirmek için bu biti ayarla. (Systick timer gibi kesmeler için gerekli)
    # FS (Floating-Point Status): Eğer FPU kullanılıyorsa Initial (0) veya Clean (1) olarak ayarlanmalı.
    # Diğer bitler (SPP, SPIE) genellikle firmware tarafından S-mode'a geçişte ayarlanır.

    # sstatus'taki SIE bitini ayarla
    li t0, (1 << 1)       # SIE biti maskesi (sstatus registerında bit 1)
    csrr t1, sstatus      # Mevcut sstatus değerini oku
    or t1, t1, t0         # SIE bitini ayarla
    csrw sstatus, t1      # Yeni değeri sstatus'a yaz

    # sie: Supervisor Interrupt Enable Register
    # STIE (Supervisor Timer Interrupt Enable): Systick timer kesmesini etkinleştir.
    # SEIE (Supervisor External Interrupt Enable): Harici cihaz kesmelerini etkinleştir.
    # SSIP (Supervisor Software Interrupt Enable): Yazılım kesmelerini etkinleştir.

    li t0, (1 << 5) | (1 << 1) | (1 << 0) # STIE (bit 5), SEIE (bit 9), SSIP (bit 1) - RISC-V spec'e göre kontrol edin! STIE 5, SEIE 9, SSIP 1.
    # li t0, (1 << 5) | (1 << 9) | (1 << 1) # STIE (bit 5), SEIE (bit 9), SSIP (bit 1) - CSR Bits spec'e göre güncellendi.
    li t0, (1 << 5) | (1 << 9) | (1 << 1) # RV64 standardı: SSIP=1, STIP=5, SEIP=9.
    csrw sie, t0          # sie CSR'ına yaz (Gerekli kesmeleri etkinleştir)


    # 5. Ana Rust Kernel Fonksiyonuna Atlama/Çağırma
    # main_kernel/init/main.rs içindeki kernel_main fonksiyonunu çağır.
    # Firmware'dan gelen argümanlar (hartid=a0, dtb_address=a1) zaten doğru registerlarda.
    call kernel_main      # kernel_main'i çağır (dönmemesi beklenir)

    # Eğer kernel_main beklenmedik bir şekilde dönerse, sistemi durdur.
    # Bu asla olmamalıdır.
halt:
    wfi # İşlemciyi uykuya al (kesme bekler)
    j halt # Sonsuz döngüde kal


# TODO: Sekonder hart başlatma fonksiyonu (çok çekirdekli sistemler için)
# Eğer SiFive S21 tek çekirdekliyse bu bölüm atlanabilir.
# Birden fazla hart varsa, başkaları buraya atlayabilir ve sadece hart 0 main_kernel'e devam eder.
# .global secondary_hart_rust_entry
# secondary_hart_rust_entry:
#   # İkincil hartlar için yığın ayarı (her hart kendi yığınına sahip olmalı)
#   # Farklı bir stack adresi kullanın (__secondary_stack_top gibi)
#   la sp, __secondary_stack_top
#   # İkincil hartlar için minimal init (CSRs, stvec gibi)
#   # Genellikle bir bekleme döngüsüne girerler ve ana hart onları uyandırana kadar beklerler.
#   # call secondary_hart_main_rust # Eğer ikincil hartlar için Rust fonksiyonu varsa
#   loop:
#      wfi
#      j loop