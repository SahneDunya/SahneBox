// main_kernel/exit.rs
// Görev (Task/Process) sonlandırma

use crate::sched::{self, TaskState}; // scheduler modülünü içeri aktar
use core::alloc::{GlobalAlloc, Layout}; // Bellek serbest bırakma için (isteğe bağlı)
use alloc::sync::Arc; // Arc kullanılıyorsa
use spin::Mutex; // Mutex kullanılıyorsa

// TODO: exit sistem çağrısı handler'ı buradan çağırabilir.
// Genellikle uygulamalar bir dönüş kodu (status) ile exit çağırır.
// Bu dönüş kodu sys_exit fonksiyonuna parametre olarak gelecektir.
#[no_mangle] // Sistem çağrısı tablosunda kullanılabilir
pub extern "C" fn sys_exit(status: i32) {
    // Şu anda çalışan görevi al
    let current_task_arc = match sched::current_task() {
        Some(task) => task,
        None => {
            // Hata: Çalışan bir görev yok. Panic veya çok temel bir hata göster.
            // Çok düşük seviyede panik genellikle sistemi durdurur.
            // printk!("exit hatası: Çalışan görev yok!\n");
            loop {} // Sistem donar
        }
    };

    // Görevin durumunu Exited olarak işaretle
    let mut current_task = current_task_arc.lock();
    current_task.state = TaskState::Exited;
    // TODO: Dönüş kodunu (status) bir yere kaydetmek gerekebilir (örn. parent process'in wait çağrısı için).
     current_task.exit_status = status;

    // TODO: Görev için ayrılan yığın belleğini serbest bırak.
    // Bu, görev yapısındaki stack alanını alıp, kullanılan bellek yöneticisi
    // aracılığıyla belleği iade etmeyi gerektirir.
     if let Some(stack_box) = current_task.stack.take() { // stack alanını move et
         let layout = Layout::from_size_align(stack_box.len(), core::mem::align_of::<u8>()).unwrap(); // Doğru Layout'u oluştur
         unsafe {
              // GLOBAL_ALLOCATOR.dealloc(Box::into_raw(stack_box) as *mut u8, layout); // Global Allocator kullanarak serbest bırak
         }
     }

    // Görev listesinden görevi kaldırma veya işaretleme.
    // Görev listesinden Arc'ı kaldırmak referans sayısını düşürecektir.
    // Eğer başka referans (örn. parent'ın task listesi) yoksa bellek serbest kalır (Arc drop edildiğinde).
    // Ancak genellikle zamanlayıcı, Exited durumundaki görevleri periyodik olarak temizler.
    // Basit bir yaklaşım, listede tutmaya devam edip sadece durumunu Exited yapmak ve zamanlayıcının bunları atlamasını sağlamaktır.
    // Daha sonra bir "reaper" görevi bu Exited görevleri listeden temizleyebilir.
    // Şu anki basit implementasyonda, durumu Exited yapmak yeterli.

    // MutexGuard'ı serbest bırak
    drop(current_task);
    drop(current_task_arc); // Arc'ın referans sayısını düşür

    // Zamanlayıcıyı çağır. Çalışmaya hazır bir sonraki göreve geçilir.
    // Bu fonksiyondan asla dönülmez, çünkü geçerli görev sonlanmıştır.
    sched::schedule();

    // schedule() fonksiyonundan dönülmemesi beklenir. Eğer dönerse bir hata var demektir.
    // Çok düşük seviyede sonsuz döngü veya panik.
     printk!("exit hatası: schedule'dan dönüldü!\n");
    loop {}
}

// TODO: Görevler bittiğinde otomatik olarak çağrılacak bir wrapper fonksiyonu gerekebilir.
// Task::new fonksiyonunda context.ra alanına bu wrapper'ın adresi yazılır.
// Görev fonksiyonu normal şekilde tamamlandığında bu wrapper'a döner ve wrapper sys_exit'i çağırır.
 #[no_mangle]
 extern "C" fn task_exit_wrapper(exit_code: i32) { // Görev fonksiyonundan gelen dönüş kodu parametre olabilir
     sys_exit(exit_code);
 }