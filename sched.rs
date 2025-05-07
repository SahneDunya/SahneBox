// main_kernel/sched.rs
// Görev (Task/Process) Yönetimi ve Zamanlayıcı

use core::fmt;
use spin::Mutex; // spin crate'i
use alloc::boxed::Box; // Heap tahsisi için alloc crate'i
use alloc::vec::Vec; // Dinamik boyutlu liste için alloc crate'i
use alloc::sync::Arc; // Birden fazla yerden referans vermek için (isteğe bağlı)

// TODO: Context Switch Assembly fonksiyonunun imzası.
// Bu fonksiyon mevcut bağlamı old_context_ptr'a kaydeder,
// new_context_ptr'dan yeni bağlamı yükler ve oradan yürütmeye devam eder.
// Rust'ta 'extern "C"' ile tanımlanmalıdır.
extern "C" {
    fn context_switch(old_context_ptr: *mut TaskContext, new_context_ptr: *const TaskContext);
}

// Görev durumu enum'u
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Runnable, // Çalışmaya hazır
    Running,  // Şu anda çalışıyor
    Blocked,  // Bir kaynağı bekliyor (basit implementasyonda kullanılmayabilir)
    Exited,   // Tamamlandı
}

// Görevin CPU bağlamını saklayan yapı
// RISC-V 64-bit (RV64) makine moduna özgü register'ları içermelidir.
#[repr(C)] // C uyumlu bellek düzeni sağlamak için
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    // Saved registers (RISC-V calling convention & caller-saved)
     x1 (ra - return address)
     x5-x7 (t0-t2 - temporaries)
     x10-x17 (a0-a7 - arguments/return values)
     x28-x31 (t3-t6 - temporaries)
    ra: usize,
    t0: usize, t1: usize, t2: usize,
    a0: usize, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, a7: usize,
    t3: usize, t4: usize, t5: usize, t6: usize,

    // Callee-saved registers (generally saved/restored by the function being called,
    // but needed for context switch to restore caller's state)
     x8 (s0/fp - frame pointer)
     x9 (s1 - saved register)
     x18-x27 (s2-s11 - saved registers)
    s0: usize,
    s1: usize,
    s2: usize, s3: usize, s4: usize, s5: usize, s6: usize, s7: usize, s8: usize, s9: usize, s10: usize, s11: usize,

    // Stack Pointer
    sp: usize, // x2

    // Program Counter (Return from trap/context switch)
    mepc: usize, // Machine Exception Program Counter - Trap'ten/Geçişten dönülecek adres

    // Machine Status Register (İnterrupt durumu vb.)
    mstatus: usize, // Machine Status Register

    // Diğer CSR'lar veya durumlar eklenebilir
    // 예를 들어, sstatus, satp (paging kullanılıyorsa)
}

impl TaskContext {
    // Boş/sıfırlarla dolu yeni bir bağlam oluşturur.
    // Gerçek başlatma Task yaratılırken yapılır.
    pub const fn empty() -> Self {
        TaskContext {
            ra: 0, t0: 0, t1: 0, t2: 0, a0: 0, a1: 0, a2: 0, a3: 0, a4: 0, a5: 0, a6: 0, a7: 0,
            t3: 0, t4: 0, t5: 0, t6: 0, s0: 0, s1: 0, s2: 0, s3: 0, s4: 0, s5: 0, s6: 0, s7: 0,
            s8: 0, s9: 0, s10: 0, s11: 0, sp: 0, mepc: 0, mstatus: 0,
        }
    }
}

// Görev Yapısı
pub struct Task {
    id: usize,
    state: TaskState,
    context: TaskContext,
    // TODO: Yığın (stack) için tahsis edilen belleği tutmak gerek.
    // Box<[u8]> veya başka bir pointer türü olabilir.
    // Bu bellek Task silindiğinde serbest bırakılmalıdır.
    stack: Option<Box<[u8]>>,
    // Diğer görev bilgileri eklenebilir (öncelik, isim vb.)
}

impl Task {
    // Yeni bir görev oluşturur.
    // entry_point: Görevin başlayacağı fonksiyonun adresi.
    // stack_size: Görev için ayrılacak yığın boyutu.
    // TODO: Bellek tahsisi (alloc) burada kullanılır. mm/memory.rs'nin çalışıyor olması gerekir.
    pub fn new(id: usize, entry_point: usize, stack_size: usize) -> Result<Self, &'static str> {
        // TODO: Yığın için bellek tahsis et.
        let stack = alloc::vec![0u8; stack_size].into_boxed_slice(); // Basit vektör tahsisi
        // TODO: Bellek tahsisi başarısız olursa hata döndür.
        // alloc::vec! başarısız olursa panic eder, daha sağlam bir alloc mekanizması gerekebilir.

        let stack_top = stack.as_ptr() as usize + stack_size;

        let mut context = TaskContext::empty();
        // Görevin başlayacağı adres (entry_point) mepc registerına yazılır.
        context.mepc = entry_point;
        // Görevin yığın göstericisi (stack pointer - sp) ayarlanır.
        // Yığınlar genellikle yüksek adresten düşük adrese doğru büyür.
        context.sp = stack_top; // Yığının en üst adresi

        // mstatus registerı, görev makine modunda çalışacaksa uygun şekilde ayarlanır.
        // Örneğin, kesmelerin etkinleştirilmesi veya devre dışı bırakılması.
        // MPIE (Machine Previous Interrupt Enable) biti genellikle 1 yapılır ki,
        // Trap'ten döndüğümüzde kesmeler etkin olsun. MPP (Machine Previous Privilege)
        // genellikle Machine modunda çalışmak için 3 yapılır.
        // TODO: Bu ayarlar çekirdeğinizin ayrıcalık seviyesi modeline göre değişir.
        const MSTATUS_MPIE: usize = 1 << 7;
        const MSTATUS_MPP_MACHINE: usize = 3 << 11;
        context.mstatus = MSTATUS_MPIE | MSTATUS_MPP_MACHINE; // Örnek ayar

        // Görev bağlamı için ra (return address) ayarlanabilir.
        // Görev fonksiyonu bittiğinde ne olacağını belirler. Genellikle bir "exit" fonksiyonuna döner.
        // TODO: Bu adresi, görev tamamlandığında çağrılacak bir task_exit_wrapper fonksiyonunun adresi yapın.
        // Bu wrapper fonksiyonu, görevden gelen dönüş değerini alıp sys_exit'i çağırmalıdır.
        // Şimdilik 0 yapalım, ama bu görev bitince crash olmasına neden olur.
        context.ra = 0; // Görev bitince dönülecek adres (geçici olarak 0)

        Ok(Task {
            id,
            state: TaskState::Runnable,
            context,
            stack: Some(stack), // Yığın belleğini sakla
        })
    }
}

// Çekirdekteki tüm görevleri tutan global liste.
// Mutex ile korunmalı.
static TASKS: Mutex<Vec<Arc<Mutex<Task>>>> = Mutex::new(Vec::new());

// Şu anda çalışan görevin ID'si.
// Başlangıçta -1 veya özel bir değer olabilir ( scheduler görevi gibi).
// Mutex ile korunmalı.
static CURRENT_TASK_ID: Mutex<Option<usize>> = Mutex::new(None);

// Zamanlayıcıyı (scheduler) başlatır. İlk görevi çalıştırır.
// İlk görev genellikle init/main.rs'deki çekirdek ana döngüsü olur.
pub fn init() {
    // TASKS vektörünü ve CURRENT_TASK_ID'yi başlatır.
    // İlk görevi (çekirdek ana döngüsü) burada oluşturup kuyruğa ekleyin.
     let initial_task_id = add_task(...); // İlk görevi ekle
     *CURRENT_TASK_ID.lock() = Some(initial_task_id);
     printk!("Zamanlayıcı başlatıldı.\n");
}

// Yeni bir görevi görev kuyruğuna ekler.
pub fn add_task(task: Task) -> usize {
    let task_id = task.id;
    let task_arc = Arc::new(Mutex::new(task));
    TASKS.lock().push(task_arc);
    task_id
}

// Şu anda çalışan görevi döndürür (Arc<Mutex<Task>> olarak).
// Dikkat: Çağıranın kilidi serbest bırakması veya MutexGuard ile çalışması gerekir.
pub fn current_task() -> Option<Arc<Mutex<Task>>> {
    let task_id_lock = CURRENT_TASK_ID.lock();
    task_id_lock.and_then(|id| {
        // TASKS listesine erişmek için başka bir kilit gerekir.
        // Dikkat: İç içe kilitlenme (deadlock) riskine dikkat!
        // Genellikle CURRENT_TASK_ID kilidi tutulurken TASKS kilidi alınmaz.
        // Farklı bir strateji (örn. index kullanarak erişim) veya kilit sırası (locking order) belirlenmelidir.
        // Basitlik için, burada TASKS listesindeki Arc clone edilir (atomic ref counting).
        let tasks_lock = TASKS.lock();
        tasks_lock.get(id).cloned() // Arc clone edilir
    })
}

// Zamanlama fonksiyonu. Çalışmaya hazır bir sonraki görevi seçer ve bağlam değiştirir.
// Bu fonksiyon ya periyodik olarak (örn. timer kesmesiyle) ya da bir görev beklemeye geçtiğinde çağrılır.
#[no_mangle] // Kesme işleyicisi veya sistem çağrısından çağrılabilir
pub fn schedule() {
    let mut tasks_lock = TASKS.lock();
    let mut current_task_id_lock = CURRENT_TASK_ID.lock();

    let old_task_id = current_task_id_lock.unwrap(); // Varsayım: Her zaman çalışan bir görev var

    // Bir sonraki runnable görevi bul (basit round-robin)
    let total_tasks = tasks_lock.len();
    let start_index = (old_task_id + 1) % total_tasks;
    let mut next_task_id = old_task_id;

    for i in 0..total_tasks {
        let candidate_id = (start_index + i) % total_tasks;
        let candidate_task_arc = &tasks_lock[candidate_id]; // Arc clone etmeden referans al
        let candidate_task = candidate_task_arc.lock(); // Görev Mutex'ini kilitle

        if candidate_task.state == TaskState::Runnable {
            next_task_id = candidate_id;
            break; // İlk runnable görevi bulduk
        }
        // Kiliti serbest bırak
        drop(candidate_task);
    }

    // Eğer çalıştırılabilir başka görev bulamadıysak (sadece geçerli görev runnable ise),
    // geçerli görevi çalıştırmaya devam et.
    if next_task_id == old_task_id {
        return;
    }

    let old_task_arc = tasks_lock[old_task_id].clone(); // Eski görevin Arc'ını al
    let new_task_arc = tasks_lock[next_task_id].clone(); // Yeni görevin Arc'ını al

    // Geçerli görevin durumunu güncelle
    old_task_arc.lock().state = TaskState::Runnable;
    new_task_arc.lock().state = TaskState::Running;

    // CURRENT_TASK_ID'yi güncellemeden kilitleri serbest bırak!
    // Context switch'ten döndüğümüzde (yeni görevde) CURRENT_TASK_ID'nin güncel olması gerekir.
    // Bu, context_switch Assembly rutininden sonra Rust'a dönüldüğünde yapılmalıdır,
    // veya Assembly rutini kendisi bu güncellemeyi yapmalıdır (çok zor).
    // Daha iyi bir yaklaşım, CURRENT_TASK_ID'yi context_switch'ten ÖNCE ayarlamak
    // ve context_switch'in eski görev bağlamını kaydederken bu yeni ID'yi kullanmasını sağlamaktır.
    // Ancak bu, Assembly'de çok daha karmaşık state yönetimi gerektirir.
    // Basitlik için, Rust'ta ID'yi güncelleyip kilitleri serbest bırakalım.
    // Kilitler, context_switch çağrılmadan önce serbest bırakılmalıdır, aksi takdirde deadlock olur.
    *current_task_id_lock = Some(next_task_id);

    // Kilitleri serbest bırak
    drop(current_task_id_lock);
    drop(tasks_lock);

    // Context Switch'i çağır
    // Güvenli olmayan (unsafe) çünkü doğrudan bellek adresleri ve Assembly fonksiyonu kullanılıyor.
    unsafe {
        let old_context_ptr = &mut old_task_arc.lock().context as *mut TaskContext;
        let new_context_ptr = &new_task_arc.lock().context as *const TaskContext;
        context_switch(old_context_ptr, new_context_ptr);
        // context_switch'ten döndüğümüzde, artık new_task_arc'ın bağlamındayız.
        // Bu noktada devam eden kod, yeni görevin kaldığı yerden devam eder.
        // Yani, aslında buradan sonraki kod yeni görev için çalışır!
        // Bu nedenle, context_switch'ten dönen kodun her iki görevde de mantıklı olması gerekir.
        // Genellikle context_switch'ten dönen yer, zamanlayıcının çağrıldığı yerdir.
    }

    // Context switch'ten sonraki kod (artık yeni görev bağlamında çalışıyor)
    // Burada, zamanlayıcı fonksiyonunun devamı gibi düşünebiliriz.
    // Ancak genellikle context_switch doğrudan zamanlayıcının çağrıldığı noktaya döner.
}

// Bir görev kendiliğinden (cooperatively) zamanlayıcıyı çağırabilir.
#[allow(dead_code)] // Eğer preemptive scheduling kullanılıyorsa bu kullanılmayabilir.
pub fn task_yield() {
    schedule();
}

// Görev listesini hata ayıklama için yazdırma (isteğe bağlı)
#[allow(dead_code)] // Kullanılmıyorsa uyarı vermemesi için
pub fn debug_print_tasks() {
    let tasks_lock = TASKS.lock();
    let current_task_id_lock = CURRENT_TASK_ID.lock();
    let current_id = current_task_id_lock.unwrap_or(usize::MAX);

    printk!("--- Görev Listesi ---\n");
    for task_arc in tasks_lock.iter() {
        let task = task_arc.lock();
        let current_marker = if task.id == current_id { "*" } else { "" };
        printk!("ID: {} Durum: {:?} {}\n", task.id, task.state, current_marker);
        // printk!("  Context: {:?}\n", task.context); // Çok detaylı olabilir
    }
    printk!("---------------------\n");
}