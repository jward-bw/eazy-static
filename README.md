# eager + lazy = eazy

A lazy-static clone that provides eager functionality for ease-of-use.

Wherever this macro is used, a public function `init_all` will be in scope, which when called access each of the variables defined in the macro block. This will initialise every variable that has not already been initialised.

```rust
use eazy_static::eazy_static;

use std::sync::{atomic::{AtomicBool, Ordering}};

static X: AtomicBool = AtomicBool::new(true);
static Y: AtomicBool = AtomicBool::new(true);

assert!(X.load(Ordering::SeqCst));
assert!(Y.load(Ordering::SeqCst));

eazy_static!{
    static ref XEDITED: &'static str = {
        X.store(false, Ordering::SeqCst);
        "X has been edited!"
    };
    static ref YEDITED: &'static str = {
        Y.store(false, Ordering::SeqCst);
        "Y has been edited!"
    };
}

assert!(X.load(Ordering::SeqCst));
assert!(Y.load(Ordering::SeqCst));

println!("{}", *XEDITED);
assert_eq!(X.load(Ordering::SeqCst), false);
assert!(Y.load(Ordering::SeqCst));

init_all();
assert_eq!(Y.load(Ordering::SeqCst), false);
```
