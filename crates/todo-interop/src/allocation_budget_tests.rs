use super::*;

#[test]
fn disabled_mode_has_no_budget_accounting() {
    let allocator = ParserAllocator::new();
    assert!(allocator.reserve(usize::MAX));
    assert!(allocator.reserve(usize::MAX));
    assert_eq!(allocator.remaining.load(Ordering::Relaxed), usize::MAX);
}

#[test]
fn an_enabled_budget_cannot_be_reset_or_overflowed() {
    let allocator = ParserAllocator::new();
    allocator.enable(128).unwrap();
    assert!(allocator.enable(256).is_err());
    assert!(!allocator.reserve(usize::MAX));
    assert!(allocator.reserve(128));
    assert!(!allocator.reserve(1));
}

#[test]
fn oversized_decoders_never_reach_the_system_allocator() {
    let allocator = ParserAllocator::new();
    allocator.enable(1024).unwrap();
    let huge = Layout::from_size_align(128 * 1024 * 1024, 8).unwrap();
    assert!(unsafe { allocator.alloc(huge) }.is_null());
    assert!(unsafe { allocator.alloc_zeroed(huge) }.is_null());
    assert_eq!(allocator.remaining.load(Ordering::Relaxed), 1024);
}

#[test]
fn deallocation_does_not_refund_cumulative_budget() {
    let allocator = ParserAllocator::new();
    allocator.enable(128).unwrap();
    let layout = Layout::from_size_align(64, 8).unwrap();
    for _ in 0..2 {
        let ptr = unsafe { allocator.alloc(layout) };
        assert!(!ptr.is_null());
        unsafe { allocator.dealloc(ptr, layout) };
    }
    assert!(unsafe { allocator.alloc(layout) }.is_null());
}

#[test]
fn zeroed_allocations_are_zeroed_and_accounted() {
    let allocator = ParserAllocator::new();
    allocator.enable(16).unwrap();
    let layout = Layout::from_size_align(16, 8).unwrap();
    let ptr = unsafe { allocator.alloc_zeroed(layout) };
    assert!(!ptr.is_null());
    assert_eq!(unsafe { std::slice::from_raw_parts(ptr, 16) }, &[0; 16]);
    unsafe { allocator.dealloc(ptr, layout) };
    assert!(!allocator.reserve(1));
}

#[test]
fn rejected_reallocation_preserves_the_original_block() {
    let allocator = ParserAllocator::new();
    allocator.enable(96).unwrap();
    let before = Layout::from_size_align(32, 8).unwrap();
    let after = Layout::from_size_align(64, 8).unwrap();
    let ptr = unsafe { allocator.alloc(before) };
    assert!(!ptr.is_null());
    unsafe { ptr.write(42) };
    let ptr = unsafe { allocator.realloc(ptr, before, 64) };
    assert!(!ptr.is_null());
    assert!(unsafe { allocator.realloc(ptr, after, 128) }.is_null());
    assert_eq!(unsafe { ptr.read() }, 42);
    unsafe { allocator.dealloc(ptr, after) };
}

#[test]
fn concurrent_reservations_never_exceed_the_budget() {
    let allocator = std::sync::Arc::new(ParserAllocator::new());
    allocator.enable(1024).unwrap();
    let workers = (0..16)
        .map(|_| {
            let allocator = allocator.clone();
            std::thread::spawn(move || (0..100).filter(|_| allocator.reserve(1)).count())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        workers
            .into_iter()
            .map(|t| t.join().unwrap())
            .sum::<usize>(),
        1024
    );
    assert_eq!(allocator.remaining.load(Ordering::Relaxed), 0);
}
