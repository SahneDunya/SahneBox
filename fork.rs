// main_kernel/fork.rs
// Yeni görev (task) yaratma işlevselliği

use alloc::sync::Arc; // Arc kullanılıyorsa
use spin::Mutex; // Mutex kullanılıyorsa
use crate::sched::{self, Task, TaskState, TaskContext}; // scheduler modülünü içeri aktar

// TODO: fork sistem çağrısı handler'ı buradan çağırabilir.
// Sistem çağrı mekanizması (traps.rs, sys.rs) üzerinden erişilecektir.
#[no_mangle] // Sistem çağrısı tablosunda kullanılabilir
pub extern "C" fn sys_fork() -> usize {
    // Şu anda çalışan görevi al (MutexGuard döndüren bir helper fonksiyon kullanılabilir)
    let current_task_arc = match sched::current_task() {
        Some(task) => task,
        None => {
            // Hata: Çalışan bir görev yok.
            // printk!("fork hatası: Çalışan görev yok!\n");
            return usize::MAX; // Unix'te -1 veya hata kodu döner, burada usize::MAX
        }
    };
    let mut current_task = current_task_arc.lock();

    // Yeni görev için bir ID ata
    // TODO: Görev ID yönetimi için daha sağlam bir mekanizma gerekebilir.
    let new_task_id = {
        let tasks_lock = sched::TASKS.lock();
        tasks_lock.len() // Basitçe listenin boyutu + 1 olabilir
    };

    // Yeni görev için yığın boyutu (mevcut görevle aynı olabilir veya farklı boyutta tahsis edilebilir)
    // TODO: Yığın boyutu stratejisini belirleyin.
    let new_stack_size = 4096; // Örnek yığın boyutu (4KB)

    // Yeni bir görev yapısı oluştur
    // Yeni görevin başlayacağı adres...
    // Unix fork'ta çocuk, fork çağrısından hemen sonra (parent ile aynı adreste) yürütmeye başlar.
    // Bunun için child task'ın mepc'si parent'ın mepc'sine ayarlanır.
    // Ayrıca child task'ın registerları parent'ınkilerin bir kopyası olmalıdır.
    // Özellikle a0 registerı child'da 0 olmalıdır (fork'tan dönüş değeri).
    let mut new_task = match Task::new(new_task_id, current_task.context.mepc, new_stack_size) {
         Ok(task) => task,
         Err(_) => {
             // Hata: Bellek tahsisi başarısız oldu.
              printk!("fork hatası: Bellek tahsisi başarısız!\n");
             return usize::MAX; // Hata kodu
         }
    };

    // Parent görev bağlamını yeni görevin bağlamına kopyala (shallow copy)
    // Bu, register durumunu kopyalar.
    new_task.context = current_task.context;

    // Çocuk görev için a0 registerını 0 olarak ayarla (fork dönüş değeri convention)
    new_task.context.a0 = 0;

    // TODO: Parent görev için a0 registerını çocuğun ID'si olarak ayarla.
    // Bu, şu anki 'current_task' kilidi tutulurken yapılabilir.
    current_task.context.a0 = new_task_id; // Parent'ın dönüş değeri child ID'si olacak

    // Yeni görevi runnable (çalışmaya hazır) duruma getir
    new_task.state = TaskState::Runnable;

    // Yeni görevi zamanlayıcının görev listesine ekle
    let new_task_id_returned = sched::add_task(new_task);

    // TODO: Yığın belleğini kopyala?
    // Unix fork'un "copy-on-write" veya tam kopya semantiği burada çok zor.
    // Eğer görevler aynı adres alanını paylaşıyorsa, yığın belleğini kopyalamak
    // genellikle hala istenir, aksi takdirde parent ve child aynı yığını kullanır ki bu tehlikelidir.
    // Fakat 2MB RAM ve basit MM ile yığını kopyalamak ciddi bir zorluktur ve muhtemelen
    // projenin başlangıç aşamasında atlanacaktır. Bu durumda, child task'ın
    // kendi yeni yığını olacak ve parent'ın yığındaki değerlerini görmeyecektir (yığın dışındaki global/heap verilerini paylaşır).
    // Bu, "thread-like" bir fork olur.

    // Parent görev MutexGuard'ını serbest bırak
    drop(current_task);
    // tasks_lock sched modülünde alındı/bırakıldıysa burada almaya gerek yok.

    // Yeni görevin ID'sini döndür (parent'ın fork çağrısından dönüş değeri)
    new_task_id_returned
}

// Basit bir görev yaratma fonksiyonu (fork'tan daha basit)
// Doğrudan bir entry point fonksiyonunu alır ve yeni görev olarak başlatır.
// TODO: fork yerine başlangıçta bu daha kullanışlı olabilir.
#[allow(dead_code)] // Kullanılmıyorsa uyarı vermemesi için
pub fn create_new_task(entry_point: fn(), stack_size: usize) -> Result<usize, &'static str> {
     // Yeni görev için bir ID ata
    let new_task_id = {
        let tasks_lock = sched::TASKS.lock();
        tasks_lock.len()
    };

    // Yeni bir görev yapısı oluştur
    // entry_point fonksiyonunun adresini al
    let entry_address = entry_point as *const () as usize;

    let new_task = Task::new(new_task_id, entry_address, stack_size)?;

    // Yeni görevi zamanlayıcının görev listesine ekle
    let new_task_id_returned = sched::add_task(new_task);

    Ok(new_task_id_returned)
}

// TODO: Bu modülün init fonksiyonu olabilir,
// ancak genellikle zamanlayıcı (sched) init edilirken ilk görevler yaratılır.
 pub fn init() {}