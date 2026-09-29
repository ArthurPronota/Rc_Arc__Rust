# Rc vs Arc в Rust

## Что общего

**`Rc<T>`** и **`Arc<T>`** — обе реализации **shared-ownership** (разделяемого владения) через **подсчёт ссылок**. Позволяют **нескольким** владельцам **одновременно** держать **одно** значение. При **последнем** `drop` — **значение** уничтожается.

## Разница: счётчик

| | `Rc<T>` | `Arc<T>` |
|---|---|---|
| **Счётчик** | **Неатомарный** `usize` | **Атомарный** `AtomicUsize` |
| **Операции** | Обычные (`+=`, `-=`) | **Атомарные** (`fetch_add`, `fetch_sub`) |
| **Скорость** | **Быстрее** | Медленнее |
| **`Send`** | ❌ Нет | ✅ Да (если `T: Send + Sync`) |
| **`Sync`** | ❌ Нет | ✅ Да (если `T: Send + Sync`) |
| **Многопоточность** | ❌ Нет | ✅ Да |

**Ключевое:** `Rc` **нельзя** отправлять в **другой** поток — **компилятор** не даст.

## `Rc<T>` — однопоточный

```rust
use std::rc::Rc;

let v = Rc::new(vec![1, 2, 3]);
let v2 = Rc::clone(&v);

println!("{}", Rc::strong_count(&v));   // 2
```

### Попытка отправить в поток

```rust
use std::rc::Rc;
use std::thread;

let v = Rc::new(vec![1, 2, 3]);

thread::spawn(move || {   // ❌ ошибка
    println!("{:?}", v);
});
```

**Ошибка:**

```
error[E0277]: `Rc<Vec<i32>>` cannot be sent between threads safely
```

**Причина:** `Rc<T>: !Send` — неатомарный счётчик.

### Когда использовать

- **Однопоточный** код.
- **Разделяемое владение** без потоков.
- **Производительность** важнее.

## `Arc<T>` — многопоточный

```rust
use std::sync::Arc;

let v = Arc::new(vec![1, 2, 3]);
let v2 = Arc::clone(&v);

println!("{}", Arc::strong_count(&v));   // 2
```

### Использование в потоке

```rust
use std::sync::Arc;
use std::thread;

let v = Arc::new(vec![1, 2, 3]);

thread::spawn({
    let v = Arc::clone(&v);
    move || {
        println!("{:?}", v);   // ✅
    }
})
.join()
.unwrap();
```

**`Arc<Vec<i32>>: Send + Sync`** → ✅.

### Когда использовать

- **Многопоточный** код.
- **Передача** между потоками.
- **`tokio::spawn`** (требует `Send + 'static`).
- **Read-heavy** данные.

## Счётчики `strong` и `weak`

**Оба** `Rc` и `Arc` имеют **два** счётчика:

| Счётчик | Что считает | Влияет на `drop`? |
|---|---|---|
| **`strong`** | **Сильные** ссылки | ✅ Да |
| **`weak`** | **Слабые** ссылки | ❌ Нет |

### `Weak`

```rust
use std::sync::{Arc, Weak};

let v = Arc::new(42);
let weak: Weak<i32> = Arc::downgrade(&v);

println!("{}", Arc::strong_count(&v));   // 1
println!("{}", Arc::weak_count(&v));     // 1

drop(v);
// Данные уничтожены, но weak всё ещё существует

if let Some(strong) = weak.upgrade() {
    println!("{}", strong);   // не выполнится
} else {
    println!("Данные уничтожены");
}
```

**`Weak`** **не** **владеет** данными. `upgrade()` → `Option<Arc<T>>`.

### Зачем `Weak`

**Разрыв циклов** — предотвращение **утечек** памяти.

## Проблема циклов

### Утечка через `Rc`

```rust
use std::rc::Rc;
use std::cell::RefCell;

struct Node {
    next: RefCell<Option<Rc<Node>>>,
}

let a = Rc::new(Node { next: RefCell::new(None) });
let b = Rc::new(Node { next: RefCell::new(Some(Rc::clone(&a))) });

*a.next.borrow_mut() = Some(Rc::clone(&b));

// a → b → a (цикл)
// Rc-счётчики никогда не дойдут до 0
// Память утечёт
```

**Проблема:** `a` владеет `b`, `b` владеет `a` → **счётчики = 2**, **никогда** не **0**.

### Решение через `Weak`

```rust
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Node {
    next: RefCell<Option<Rc<Node>>>,
    prev: RefCell<Option<Weak<Node>>>,   // ← Weak для обратной ссылки
}

let a = Rc::new(Node {
    next: RefCell::new(None),
    prev: RefCell::new(None),
});

let b = Rc::new(Node {
    next: RefCell::new(Some(Rc::clone(&a))),
    prev: RefCell::new(Some(Rc::downgrade(&a))),   // ← Weak
});

*a.next.borrow_mut() = Some(Rc::clone(&b));
```

**`Weak`** **не** **увеличивает** `strong` → цикл **разрывается**.

## `Rc::new_cyclic` и `Arc::new_cyclic`

**Создают** значение, которое **ссылается** на **себя** через `Weak`:

```rust
use std::rc::{Rc, Weak};
use std::cell::RefCell;

struct Node {
    name: String,
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
}

let root = Rc::new_cyclic(|weak_root| Node {
    name: "root".to_string(),
    parent: RefCell::new(Weak::new()),   // root — без родителя
    children: RefCell::new(vec![]),
});

let child = Rc::new_cyclic(|weak_child| Node {
    name: "child".to_string(),
    parent: RefCell::new(Rc::downgrade(&root)),   // ← Weak на root
    children: RefCell::new(vec![]),
});

root.children.borrow_mut().push(child);
```

## Сводная таблица

| Аспект | `Rc<T>` | `Arc<T>` |
|---|---|---|
| **Счётчик** | Неатомарный | **Атомарный** |
| **Скорость** | **Быстрее** | Медленнее |
| **`Send`** | ❌ | ✅ |
| **`Sync`** | ❌ | ✅ |
| **Потоки** | ❌ | ✅ |
| **Когда** | **Однопоточно** | **Многопоточно** |
| **Атомарный инкремент** | ❌ | ✅ `Relaxed` |
| **Атомарный декремент** | ❌ | ✅ `Release`/`Acquire` |

## `Arc` внутри

**`Arc`** использует **атомарные** операции:

- **Инкремент** — `AtomicUsize::fetch_add(1, Relaxed)`.
- **Декремент** — `fetch_sub(1, Release)` + `fence(Acquire)` при **последнем**.

**Почему `Relaxed` на инкременте:** **нет** нужды **синхронизировать** — счётчик **только** увеличивается.

**Почему `Release`/`Acquire` на декременте:** **последний** `drop` должен **увидеть** все **изменения** других потоков.

## Когда что выбирать

| Ситуация | Решение |
|---|---|
| **Однопоточный** код | `Rc<T>` |
| **Многопоточный** код | `Arc<T>` |
| **`tokio::spawn`** | `Arc<T>` (требует `Send + 'static`) |
| **Read-heavy** данные | `Arc<T>` |
| **Нужна мутация** | `Arc<Mutex<T>>` или `Arc<RwLock<T>>` |
| **Циклы** | `Weak<T>` |
| **Дерево с родителями** | `Weak` для **родителей** |

## Сводная таблица

| Выбор | Когда |
|---|---|
| **`Rc<T>`** | **Один** поток |
| **`Arc<T>`** | **Много** потоков |
| **`Arc<Mutex<T>>`** | Мутация из **потоков** |
| **`Arc<RwLock<T>>`** | Много **читателей** |
| **`Weak<T>`** | **Разрыв** циклов |

## Итог

- **`Rc<T>`** и **`Arc<T>`** — **shared ownership** через **подсчёт ссылок**.
- **Разница:** **неатомарный** (`Rc`) vs **атомарный** (`Arc`) счётчик.
- **`Rc`** — **быстрее**, но **`!Send`** + **`!Sync`** → **один** поток.
- **`Arc`** — **медленнее**, но **`Send + Sync`** → **много** потоков.
- **`Weak`** — для **разрыва** циклов.
- **`new_cyclic`** — для **самореферентных** структур.
- **`tokio::spawn`** требует **`Send + 'static`** → **`Arc`**.
- **Мутация** — **`Arc<Mutex<T>>`** или **`Arc<RwLock<T>>`**.
- **В вашем примере:** `Arc::clone` → **два** владельца, **поток** читает.
- **Правило:** **один** поток → `Rc`; **много** → `Arc`.
