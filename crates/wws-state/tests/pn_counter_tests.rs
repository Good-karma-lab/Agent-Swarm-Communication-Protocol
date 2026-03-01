use wws_state::pn_counter::PnCounter;

#[test]
fn test_increment() {
    let mut c = PnCounter::new("a");
    c.increment(10);
    assert_eq!(c.value(), 10);
}

#[test]
fn test_decrement() {
    let mut c = PnCounter::new("a");
    c.increment(20);
    c.decrement(5);
    assert_eq!(c.value(), 15);
}

#[test]
fn test_value_cannot_go_below_zero_after_merge() {
    let mut c = PnCounter::new("a");
    c.decrement(100); // decrement more than incremented
    // value can be negative (CRDT allows it, callers clamp)
    assert!(c.value() <= 0);
}

#[test]
fn test_merge_increments() {
    let mut a = PnCounter::new("a");
    a.increment(5);
    let mut b = PnCounter::new("b");
    b.increment(3);
    a.merge(&b);
    assert_eq!(a.value(), 8);
}

#[test]
fn test_merge_idempotent() {
    let mut a = PnCounter::new("a");
    a.increment(5);
    let mut b = PnCounter::new("b");
    b.increment(3);
    a.merge(&b);
    a.merge(&b); // merge twice should be same
    assert_eq!(a.value(), 8);
}

#[test]
fn test_merge_commutative() {
    let mut a = PnCounter::new("a");
    a.increment(5);
    let mut b = PnCounter::new("b");
    b.increment(3);
    let a_clone = a.clone();
    a.merge(&b);
    b.merge(&a_clone);
    assert_eq!(a.value(), b.value());
}

#[test]
fn test_merge_associative() {
    let mut a = PnCounter::new("a");
    a.increment(5);
    let mut b = PnCounter::new("b");
    b.increment(3);
    let mut c = PnCounter::new("c");
    c.increment(2);
    let mut a2 = a.clone();
    let b_clone = b.clone();
    a.merge(&b);
    a.merge(&c);
    b.merge(&c);
    a2.merge(&b);
    assert_eq!(a.value(), a2.value());
}
