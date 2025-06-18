Unique owner `Rc`, like `Box`, can `Send` and `DerefMut`

# Example
```rust
use std::{rc::Rc, thread::spawn};
use unique_rc::UniqRc;

let rc = Rc::new("foo".to_owned());
let weak = Rc::downgrade(&rc);

assert_eq!(weak.upgrade(), Some(Rc::new("foo".to_owned())));

let urc = UniqRc::new(rc);

spawn(move || {
    assert_eq!(*urc, "foo");
}).join().unwrap();

assert_eq!(weak.upgrade(), None);
```

Like a `Box`, `DerefMut`

```rust
use unique_rc::UniqRc;

let mut unique_rc = UniqRc::new_value(3);
*unique_rc += 1;
assert_eq!(*unique_rc, 4);
```

# Safety
- miri test passed
