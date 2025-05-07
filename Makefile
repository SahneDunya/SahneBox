# Makefile
# SahneBox İşletim Sistemi Build Sistemi

# ==============================================================================
# Proje Ayarları
# ==============================================================================

# Hedef Mimari
ARCH := riscv64
TARGET := $(ARCH)-unknown-none-elf # Cargo target triplet
CROSS_COMPILE ?= $(ARCH)-unknown-elf- # Çapraz derleyici prefixi (örneğin binutils için as, ld)

# Build Dizini (Cargo build çıktıları buraya gider)
BUILD_DIR := target/$(TARGET)/release

# Bileşen Yolları
FIRMWARE_DIR := firmware
KERNEL_DIR := main_kernel
USER_DIR := user_space # Kullanıcı alanı uygulamaları ve kütüphaneleri için ana dizin varsayımı
# TODO: Kullanıcı alanı dizin yapısını netleştirin ve burayı güncelleyin.
# Örnek: USER_APPS := shell/sh64 package_manager/spm ...
 USER_LIBS := filesystem/ext windows_system/libsaheneui_minimal ...

# Çıktı Dosyaları
FIRMWARE_BIN := $(BUILD_DIR)/$(FIRMWARE_DIR)/firmware.bin # Firmware binary çıktısı
KERNEL_BIN := $(BUILD_DIR)/$(KERNEL_DIR)/kernel.bin       # Kernel binary çıktısı
# TODO: Kullanıcı alanı uygulamalarının binary çıktısı yolları


# İmaj Ayarları
IMAGE_FILE := sahnebox.img            # Oluşturulacak disk imajı adı
IMAGE_SIZE := 25M                     # İmaj boyutu (eMMC boyutuna yakın)
IMAGE_BLOCK_SIZE := 4096              # Dosya sistemi blok boyutu (EXT2 için)
IMAGE_DEVICE_BLOCK_SIZE := 512        # Sanal cihazın blok boyutu (Disk I/O için)

# Bootloader (Firmware) Yerleşimi
# Firmware'ın imajın başında kaç MB/KB offsetle başlayacağını belirleyin.
# Bu PacketBox'ın boot ROM'unun firmware'ı nerede aradığına bağlıdır.
# EXT2 süper bloğu genellikle 1024 offsettedir, bu yüzden ilk 1024 byte'ı üzerine yazmak OK olabilir.
# Ama tam bootloader boyutunu ve hedef cihazın boot mekanizmasını bilmek gerekir.
# Örneğin, firmware 64KB'lık bir alana sığıyorsa ve cihaz başından 64KB okuyorsa:
 FIRMWARE_OFFSET_BYTES := 0
# Veya cihaz 1MB offsetten arıyorsa:
 FIRMWARE_OFFSET_BYTES := 1024 * 1024
# Şimdilik 0 varsayalım, imajın en başına yazılacak.
FIRMWARE_OFFSET_BYTES := 0


# QEMU Ayarları
QEMU := qemu-system-$(ARCH)
QEMU_MACHINE := sifive_u # RISC-V makine tipi (SiFive S21 için uygun olabilir)
QEMU_MEM := 2M           # QEMU'ya ayrılacak RAM (PacketBox 2MB)
QEMU_ARGS := -machine $(QEMU_MACHINE) -m $(QEMU_MEM) -nographic # -nographic UART konsol kullanır
# Sanal disk olarak imajı bağla (-drive format=raw,file=...)
QEMU_ARGS += -drive format=raw,file=$(IMAGE_FILE)
# Diğer QEMU argümanları (gerekirse): -kernel $(KERNEL_BIN), -bios $(FIRMWARE_BIN) (hangisinin boot ettiği confige bağlı)
# Genellikle -bios ile firmware'ı verip, firmware'ın imajdan kernel'ı okumasını istersiniz.
# Veya QEMU'nun direkt kernel'ı yüklemesini sağlarsınız (basit test için).
# Bizim senaryomuzda firmware imajı yüklüyor, o yüzden -bios kullanılacak.
QEMU_ARGS += -bios $(FIRMWARE_BIN)


# ==============================================================================
# Hedefler (Targets)
# ==============================================================================

# Varsayılan Hedef: Her Şeyi Derle ve İmajı Oluştur
.PHONY: all
all: image

# Firmware Derleme
# Cargo build kullanır
.PHONY: firmware
firmware: $(FIRMWARE_DIR)
	@echo "-> Building Firmware..."
	@cargo build --release --target $(TARGET) --manifest-path $(FIRMWARE_DIR)/Cargo.toml
	# Cargo çıktısını yakalayıp binary'nin tam yolunu almalıyız
	# veya standart çıktıyı varsaymalıyız: target/riscv64gc-unknown-none-elf/release/firmware
	@echo "Firmware built: $(FIRMWARE_BIN)" # Bu yol Cargo.toml name fieldına bağlı

# Kernel Derleme
# Cargo build kullanır
.PHONY: kernel
kernel: $(KERNEL_DIR)
	@echo "-> Building Kernel..."
	@cargo build --release --target $(TARGET) --manifest-path $(KERNEL_DIR)/Cargo.toml
	# Cargo çıktısını yakalayıp binary'nin tam yolunu almalıyız
	# veya standart çıktıyı varsaymalıyız: target/riscv64gc-unknown-none-elf/release/main_kernel
	@echo "Kernel built: $(KERNEL_BIN)" # Bu yol Cargo.toml name fieldına bağlı


# Kullanıcı Alanı Bileşenlerini Derleme
# TODO: Kullanıcı alanı bileşenleriniz için ayrı Cargo projeleriniz (applications, libraries) olduğunu varsayalım.
# Ya her biri için ayrı cargo build çağrısı yapılır ya da bir Cargo workspace kullanılıyorsa tek çağrı yeter.
.PHONY: user
user:
	@echo "-> Building User-Space Components..."
	# TODO: Her kullanıcı alanı projesi için cargo build çağrısı ekleyin.
	# Örnek:
	# @cargo build --release --target $(TARGET) --manifest-path shell/sh64/Cargo.toml
	# @cargo build --release --target $(TARGET) --manifest-path package_manager/spm/Cargo.toml
	# ...
	@echo "User-Space Components built."


# Disk İmajı Oluşturma
# Kernel ve kullanıcı alanı dosyalarını içeren boot edilebilir disk imajını oluşturur.
# DİKKAT: Bu target root (sudo) yetkisi gerektirebilir çünkü imajı mount etmek için loop cihazı kullanır.
.PHONY: image
image: kernel user # İmaj oluşturmadan önce kernel ve user derlenmiş olmalı
	@echo "-> Creating Disk Image: $(IMAGE_FILE) ($(IMAGE_SIZE))"
	# 1. İmaj dosyası için boş yer ayır
	@dd if=/dev/zero of=$(IMAGE_FILE) bs=1M count=$(shell echo $(IMAGE_SIZE) | sed 's/M//') conv=notrunc
	# Veya daha doğru boyutu hesapla:
	# @dd if=/dev/zero of=$(IMAGE_FILE) bs=1 count=0 seek=$(IMAGE_SIZE)

	# 2. İmaj dosyası üzerinde EXT2 dosya sistemi oluştur
	# -F: Kontrol sormadan devam et
	# -b <block_size>: Blok boyutunu ayarla
	# -r 0: Ayrılmış blok sayısını 0 yap
	# -O ^metadata_csum : checksum'ı kapat (basit ext2 için)
	@echo "Formatting image with EXT2 (block size $(IMAGE_BLOCK_SIZE))..."
	@sudo mke2fs -F -t ext2 -b $(IMAGE_BLOCK_SIZE) -r 0 -O ^metadata_csum $(IMAGE_FILE) $(shell echo $(IMAGE_SIZE) | sed 's/M//')M

	# 3. İmaj dosyasını loop cihazı kullanarak mount et
	@echo "Mounting image..."
	@mkdir -p mnt
	@sudo mount -o loop $(IMAGE_FILE) mnt

	# 4. Derlenmiş Kernel ve Kullanıcı Alanı Dosyalarını İmaja Kopyala
	@echo "Copying files to image..."
	# TODO: Bu kısım, SahneBox'ın beklediği dosya sistemi yapısına göre güncellenmelidir.
	# Kernel binary'nin dosya sisteminde nerede bulunacağını belirleyin. Genellikle /boot/kernel veya /kernel
	# Kullanıcı alanı uygulamaları genellikle /bin dizinine kopyalanır.
	# Arka plan resmi /Arkaplan.jpg olarak kopyalanmalı.
	# Gerekli dizinleri oluştur
	@sudo mkdir -p mnt/boot
	@sudo mkdir -p mnt/bin
	@sudo mkdir -p mnt/etc/spm # Paket yöneticisi listesi için
	@sudo mkdir -p mnt/packages # Paket dosyaları için (installer source)

	# Derlenmiş kernel binary'yi kopyala
	@sudo cp $(KERNEL_BIN) mnt/boot/kernel.bin # Kernel binary'nin FS içindeki adı

	# TODO: Derlenmiş kullanıcı alanı uygulamalarını ve kütüphanelerini kopyala
	# Örnek:
	# @sudo cp $(BUILD_DIR)/shell/sh64/sh64 mnt/bin/
	# @sudo cp $(BUILD_DIR)/package_manager/spm/spm mnt/bin/
	# @sudo cp $(BUILD_DIR)/desktop_environment/sahnedesktop/sahnedesktop mnt/bin/
	# @sudo cp $(BUILD_DIR)/voice_server/vsd/vsd mnt/bin/
	# @sudo cp $(BUILD_DIR)/user_apps/test_gui/test_gui mnt/bin/
	# @sudo cp $(BUILD_DIR)/user_apps/test_audio/test_audio mnt/bin/
	# ... Diğer uygulamalar ...

	# Arka plan resmini kopyala (varsayılan dizinden)
	# @sudo cp path/to/Arkaplan.jpg mnt/Arkaplan.jpg # Arkaplan resminin kaynağını belirtin

	# TODO: installer source imaj dosyasını kopyala (installer.img veya sahnebox.img)
	# Bu imaj dosyasının içeriği, kurulum sırasında hedef cihaza kopyalanacak olan imajdır.
	# @sudo cp path/to/installer.img mnt/sahnebox.img # Bu sahnebox.img dosyası, oluşturduğumuz IMAGE_FILE'dan farklı, onun İÇİNDE yer alan dosya!

	# 5. İmaj dosyasını unmount et
	@echo "Unmounting image..."
	@sudo umount mnt
	@rmdir mnt

	# 6. Firmware binary'yi imajın en başına (veya belirtilen offsete) yaz
	# Bu adım, EXT2 dosya sisteminin üzerine yazabilir, ancak bootloader'ın konumu genellikle dosya sistemi başlangıcından öncedir.
	@echo "Writing firmware to image (offset $(FIRMWARE_OFFSET_BYTES))..."
	@dd if=$(FIRMWARE_BIN) of=$(IMAGE_FILE) bs=1 conv=notrunc seek=$(FIRMWARE_OFFSET_BYTES)

	@echo "Disk image '$(IMAGE_FILE)' created successfully."


# QEMU'da Çalıştırma
.PHONY: run
run: image # Çalıştırmadan önce imaj oluşturulmuş olmalı
	@echo "-> Running in QEMU..."
	@$(QEMU) $(QEMU_ARGS)

# Donanıma Flaşıma (PaketBox için donanıma özel)
.PHONY: flash
flash: firmware # Firmware flaşlanmadan önce derlenmiş olmalı
	@echo "-> Flashing Firmware to PacketBox..."
	# TODO: PacketBox için gerçek flaşlama komutunu buraya ekleyin.
	# Bu genellikle bir JTAG/SWD adaptörü ve ilgili aracı (openocd, pyocd vb.) kullanır.
	# Örnek: openocd -f interface/jlink.cfg -f target/sifive-e31.cfg -c "program $(FIRMWARE_BIN) verify reset"
	@echo "Flashing command not implemented. Please add the actual flashing command."


# Temizlik
.PHONY: clean
clean:
	@echo "-> Cleaning build artifacts..."
	@cargo clean --manifest-path $(FIRMWARE_DIR)/Cargo.toml
	@cargo clean --manifest-path $(KERNEL_DIR)/Cargo.toml
	# TODO: Diğer kullanıcı alanı projeleri için temizleme ekleyin.
	# @cargo clean --manifest-path shell/sh64/Cargo.toml
	# ...
	@rm -f $(IMAGE_FILE) # İmaj dosyasını sil
	@rm -rf mnt # Mount dizinini sil (eğer kalmışsa)
	@echo "Clean done."

# ==============================================================================
# Klasör Varlığını Sağlama (Opsiyonel, sadece make clean sorunlarını önlemek için)
# ==============================================================================
$(FIRMWARE_DIR):
	@mkdir -p $@
$(KERNEL_DIR):
	@mkdir -p $@
# TODO: Diğer kullanıcı alanı dizinleri

# ==============================================================================
# Yardımcı Fonksiyonlar / Değişkenler
# ==============================================================================
# $(shell ...) kullanarak bash komutlarının çıktısını alabiliriz.
# sed ile M harfini silmek gibi.


# ==============================================================================
# Build Süreci Notları
# ==============================================================================
# 1. Makefile 'all' hedefi çalıştırılır.
# 2. 'image' hedefi çalışır.
# 3. 'image' hedefi, bağımlılıkları olan 'kernel' ve 'user' hedeflerini çalıştırır.
# 4. 'kernel' hedefi, $(KERNEL_DIR) klasörüne gidip Cargo.toml'daki projeyi --target $(TARGET) ile derler. build.rs çalışır, Assembly derlenir, Rust derlenir, hepsi linklenip $(KERNEL_BIN) oluşur.
# 5. 'user' hedefi, tüm kullanıcı alanı projelerini derler (ayrı ayrı veya workspace).
# 6. 'image' hedefi, boş bir imaj dosyası oluşturur ($(IMAGE_FILE)).
# 7. 'image' hedefi, imajı EXT2 olarak formatlar (mke2fs).
# 8. 'image' hedefi, imajı mount eder.
# 9. 'image' hedefi, derlenmiş kernel ve user binary'leri ile diğer dosyaları mount edilmiş imaja kopyalar.
# 10. 'image' hedefi, imajı unmount eder.
# 11. 'image' hedefi, derlenmiş firmware binary'yi imaj dosyasının başlangıcına (veya $(FIRMWARE_OFFSET_BYTES) offsetine) yazar.
# 12. 'run' hedefi (çağrıldıysa), QEMU'yu belirtilen ayarlar ve oluşturulan imaj dosyası ile başlatır.