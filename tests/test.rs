#[allow(unused_imports)]
use std::{any::Any, io::{Cursor, Read}, rc::{Rc, Weak}, thread::spawn};

use unique_rc::UniqRc;

#[test]
fn send() {
    let rc = Rc::new(2);
    let weak = Rc::downgrade(&rc);

    assert_eq!(weak.upgrade(), Some(Rc::new(2)));

    let urc = UniqRc::new(rc);

    spawn(move || {
        assert_eq!(*urc, 2);
    }).join().unwrap();

    assert_eq!(weak.upgrade(), None);
}

#[test]
fn send_cloned() {
    let rc = Rc::new(2);
    let _rc2 = Rc::clone(&rc);
    let weak = Rc::downgrade(&rc);

    assert_eq!(weak.upgrade(), Some(Rc::new(2)));

    let urc = UniqRc::new(rc);

    spawn(move || {
        assert_eq!(*urc, 2);
    }).join().unwrap();

    assert_eq!(weak.upgrade(), Some(Rc::new(2)));
}

#[test]
fn inner() {
    let rc = Rc::new("a".to_owned());
    let _rc2 = Rc::clone(&rc);
    let urc = UniqRc::new(rc);

    assert_eq!(UniqRc::into_inner(urc), "a");
}

#[test]
fn clone() {
    let a = UniqRc::new(Rc::new("a".to_owned()));
    let b = a.clone();

    assert_eq!(UniqRc::into_inner(a), "a");
    assert_eq!(UniqRc::into_inner(b), "a");
}

#[test]
fn dst_from() {
    let a: UniqRc<str> = UniqRc::from("a");
    assert_eq!(a.as_ref(), "a");
}

#[test]
fn dst_clone() {
    let a: Rc<str> = Rc::from("a");
    let ua = UniqRc::new(a.clone());
    assert_eq!(a.as_ref(), "a");
    assert_eq!(ua.as_ref(), "a");
}

#[test]
fn iter() {
    let mut iter = UniqRc::new_value(0..5);
    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.nth(1), Some(2));
}

#[test]
fn pin() {
    let mut _p = UniqRc::pin(8);
}

#[test]
fn into_pin() {
    let urc = UniqRc::new_value("a".to_owned());
    let mut _p = UniqRc::into_pin(urc);
}

#[test]
#[ignore = "miri leak"]
fn leak() {
    let urc = UniqRc::new_value("foo".to_owned());
    let leak = UniqRc::leak(urc);
    assert_eq!(leak, "foo");
}

#[cfg(feature = "std")]
#[test]
fn read() {
    let mut from = UniqRc::new_value(Cursor::new(vec![1u8, 2, 3]));
    let mut buf = [0; 5];
    assert_eq!(UniqRc::read(&mut from, &mut buf).unwrap(), 3);
    assert_eq!(buf, [1, 2, 3, 0, 0]);
}

#[test]
fn downcast_fail() {
    let rc: Rc<i32> = Rc::new(3);
    let dyn_rc: Rc<dyn Any + 'static> = rc;
    let urc = UniqRc::try_new(dyn_rc).unwrap();
    assert!(urc.downcast::<i8>().is_err());
}

#[test]
fn downcast() {
    let rc: Rc<i32> = Rc::new(3);
    let dyn_rc: Rc<dyn Any + 'static> = rc;
    let urc = UniqRc::try_new(dyn_rc).unwrap();
    assert_eq!(*urc.downcast::<i32>().unwrap(), 3);
}

#[test]
fn slice_rc_from_iter() {
    let rc = UniqRc::from_iter(0..4);
    assert_eq!(*rc, [0, 1, 2, 3]);
}

#[test]
fn shared_new() {
    let rc = Rc::new(3);
    let rc1 = rc.clone();

    assert_eq!(Rc::strong_count(&rc), 2);
    assert_eq!(Rc::weak_count(&rc), 0);
    assert_eq!(Rc::strong_count(&rc1), 2);
    assert_eq!(Rc::weak_count(&rc1), 0);

    let mut unique_rc = UniqRc::new(rc);

    assert_eq!(Rc::strong_count(&rc1), 1);
    assert_eq!(Rc::weak_count(&rc1), 0);

    *unique_rc = 4;
    assert_eq!(*unique_rc, 4);
}

#[test]
fn shared_weak_new() {
    let rc = Rc::new(3);
    let weak = Rc::downgrade(&rc);

    assert_eq!(Rc::strong_count(&rc), 1);
    assert_eq!(Rc::weak_count(&rc), 1);
    assert_eq!(Weak::strong_count(&weak), 1);
    assert_eq!(Weak::weak_count(&weak), 1);

    let mut unique_rc = UniqRc::new(rc);

    assert_eq!(Weak::strong_count(&weak), 0);
    assert_eq!(Weak::weak_count(&weak), 0);

    *unique_rc = 4;
    assert_eq!(*unique_rc, 4);
}

#[test]
fn assign() {
    let mut rc = UniqRc::new_value([0, 1, 2]);
    (&mut *rc)[2] += 1;
    assert_eq!(*rc, [0, 1, 3]);
}

#[test]
#[should_panic(expected = "should not return shared")]
fn shared_uniq_rc_from_iter_fail() {
    struct Foo(Option<Weak<Self>>);
    impl FromIterator<Foo> for Rc<Foo> {
        fn from_iter<T: IntoIterator<Item = Foo>>(iter: T) -> Self {
            let mut first = iter.into_iter().next().unwrap();
            Rc::new_cyclic(|weak| {
                first.0 = weak.clone().into();
                first
            })
        }
    }
    let _: UniqRc<Foo> = UniqRc::from_iter([Foo(None)]);
}
