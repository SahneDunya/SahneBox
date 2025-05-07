# main_kernel/boot/head.S
# SahneBox Kernel Çok Erken Başlatma Kodu (S-mode)
# Firmware buradan baslar.

.section .text.entry    # Linker scriptteki özel başlangıç bölümüne yerleştir
.global _start          # Global erişilebilir yap (Firmware buradan atlar)
.align 2                # Yönerge hizalaması (4 byte)

_start:
    # Firmware'dan gelen argümanlar:
    # a0 = hartid (İşlemci çekirdek ID'si)
    # a1 = dtb_address (Device Tree Blob adresi)
    # Diğer registerlar tanımsız olabilir.

    # Çok erken başlatma (gerekirse)
    # Örneğin, eğer birden fazla hart varsa, sadece hart 0'ın tam başlatmayı yapmasını sağlayın.
    # Tek çekirdekli SiFive S21 için bu atlanabilir, ama iyi pratiktir.
    # li t0, 0          # Hart ID 0
    # bne a0, t0, secondary_hart_start # Eğer hart 0 değilse başka yere dallan (gerekirse)

    # Hart 0 ana başlatmaya devam eder
primary_hart_start:
    # Firmware'dan gelen argümanları (a0, a1) koruyarak boot.S'deki ana başlatma fonksiyonuna atla.
    # _start_rust fonksiyonuna atlar.
    j _start_rust

# Örnek: Diğer hartlar için başlangıç noktası (SiFive S21 için gerekli değil, çok çekirdekli sistemler için)
# secondary_hart_start:
    # printk!("DEBUG: İkincil hart {} başlatılıyor...\n", a0); # printk firmware'ın UART'ına bağlıdır.
    # TODO: İkincil hartlar için minimal başlatma (örneğin, sadece dur/uyu)
    # loop:
    #     wfi
    #     j loop