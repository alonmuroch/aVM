#![no_std]
#![no_main]

extern crate alloc;

// Memory mapping edge-case tests: alignment, boundary spanning, map_to_physical, mirror gaps,
// copy_user atomicity, and remap overrides.
use clibc::log;
use kernel::BootInfo;
use kernel::memory::page_allocator::{self, PagePerms};

#[path = "../../tests/results.rs"]
mod results;
#[path = "../../tests/fail.rs"]
mod fail;
#[path = "../../tests/utils.rs"]
mod utils;

const PAGE_SIZE: usize = 0x1000;
const L1_SPAN: u32 = 1 << 22;

#[unsafe(no_mangle)]
pub extern "C" fn _start(
    input_ptr: *const u8,
    input_len: usize,
    boot_info_ptr: *const BootInfo,
) {
    log!("kernel mem map edge test boot");
    let info = utils::init_test_kernel(boot_info_ptr);

    clibc::logf!("kernel test input len: %d", input_len as u32);
    let _input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };

    let user_root = page_allocator::alloc_root().unwrap_or(0);
    if user_root == 0 {
        fail::fail(1);
    }

    if let Err(code) = test_unaligned_map_and_translate(user_root, info) {
        fail::fail(code);
    }
    if let Err(code) = test_cross_l1_boundary(user_root, info) {
        fail::fail(code);
    }
    if let Err(code) = test_multiple_l2_tables(user_root, info) {
        fail::fail(code);
    }
    if let Err(code) = test_zero_len_map_no_effect(user_root, info) {
        fail::fail(code);
    }
    if let Err(code) = test_map_to_physical_alignment_and_alias(user_root, info) {
        fail::fail(code);
    }
    if let Err(code) = test_mirror_gap_behavior(user_root, info) {
        fail::fail(code);
    }
    if let Err(code) = test_copy_user_atomic(user_root, info) {
        fail::fail(code);
    }
    if let Err(code) = test_remap_override_perms(user_root, info) {
        fail::fail(code);
    }

    log!("kernel mem map edge test done");
    utils::pass();
}

fn test_unaligned_map_and_translate(user_root: u32, info: BootInfo) -> Result<(), u32> {
    // Description: map an unaligned range that crosses a page boundary and verify translations
    // and data access on the mapped pages.
    log!("test: unaligned map + translate");
    log!("subtest: map unaligned range and confirm translations are present");

    let base = pick_user_va(info, 0x4000);
    let va_start = base.wrapping_add(37);
    let len = PAGE_SIZE + 123;
    let perms = PagePerms::new(true, true, false, true);
    if !page_allocator::map_range_for_root(user_root, va_start, len, perms) {
        return Err(10);
    }

    let first_phys = page_allocator::translate(user_root, va_start).unwrap_or(0);
    let last_phys = page_allocator::translate(user_root, va_start.wrapping_add(len as u32 - 1))
        .unwrap_or(0);
    if first_phys == 0 || last_phys == 0 {
        return Err(11);
    }

    log!("subtest: write data and confirm it can be read back");
    let data = [0x12u8, 0x34, 0x56, 0x78];
    if !page_allocator::copy(user_root, va_start, &data) {
        return Err(12);
    }
    let word = page_allocator::peek_word(user_root, va_start).unwrap_or(0);
    if word != 0x7856_3412 {
        return Err(13);
    }
    Ok(())
}

fn test_cross_l1_boundary(user_root: u32, info: BootInfo) -> Result<(), u32> {
    // Description: map a range that crosses a VPN1 boundary and ensure both pages are mapped
    // and writable.
    log!("test: cross L1 boundary mapping");
    log!("subtest: pick a range that crosses a 4 MiB boundary");

    let window_start = info.va_base;
    let window_end = info.va_base.saturating_add(info.va_len);
    let next_boundary = align_up_u32(window_start.saturating_add(0x1000), L1_SPAN);
    let start = next_boundary.saturating_sub(PAGE_SIZE as u32);
    let end = start.saturating_add((PAGE_SIZE * 2) as u32);
    if start < window_start || end > window_end {
        log!("subtest: skipped (window too small for boundary test)");
        return Ok(());
    }

    let perms = PagePerms::new(true, true, false, true);
    if !page_allocator::map_range_for_root(user_root, start, PAGE_SIZE * 2, perms) {
        return Err(20);
    }

    log!("subtest: confirm translations and data access on both pages");
    let first_phys = page_allocator::translate(user_root, start).unwrap_or(0);
    let second_phys = page_allocator::translate(user_root, start.wrapping_add(PAGE_SIZE as u32))
        .unwrap_or(0);
    if first_phys == 0 || second_phys == 0 {
        return Err(21);
    }

    let first_data = [0xa1u8, 0xa2, 0xa3, 0xa4];
    let second_data = [0xb1u8, 0xb2, 0xb3, 0xb4];
    if !page_allocator::copy_user(user_root, start, &first_data) {
        return Err(22);
    }
    if !page_allocator::copy_user(user_root, start.wrapping_add(PAGE_SIZE as u32), &second_data) {
        return Err(23);
    }
    let first_word = page_allocator::peek_word(user_root, start).unwrap_or(0);
    let second_word = page_allocator::peek_word(user_root, start.wrapping_add(PAGE_SIZE as u32))
        .unwrap_or(0);
    if first_word != 0xa4a3_a2a1 || second_word != 0xb4b3_b2b1 {
        return Err(24);
    }
    Ok(())
}

fn test_zero_len_map_no_effect(user_root: u32, info: BootInfo) -> Result<(), u32> {
    // Description: mapping a zero-length range should be a no-op.
    log!("test: zero-length map is a no-op");
    log!("subtest: ensure translation stays absent");

    let va = pick_user_va(info, 0x9000);
    if page_allocator::translate(user_root, va).is_some() {
        return Err(30);
    }
    let perms = PagePerms::new(true, true, false, true);
    if !page_allocator::map_range_for_root(user_root, va, 0, perms) {
        return Err(31);
    }
    if page_allocator::translate(user_root, va).is_some() {
        return Err(32);
    }
    Ok(())
}

fn test_multiple_l2_tables(user_root: u32, info: BootInfo) -> Result<(), u32> {
    // Description: map pages in multiple VPN1 regions to ensure more than two L2 tables
    // can be allocated and accessed.
    log!("test: multiple L2 tables via sparse VPN1 mapping");
    log!("subtest: map one page in three distinct VPN1 regions");

    let window_start = info.va_base;
    let window_end = info.va_base.saturating_add(info.va_len);
    let first_region = align_up_u32(window_start.saturating_add(0x1000), L1_SPAN);
    let third_region = first_region.saturating_add(L1_SPAN * 2);
    if third_region.saturating_add(PAGE_SIZE as u32) > window_end {
        log!("subtest: skipped (window too small for multi-L2 test)");
        return Ok(());
    }

    let perms = PagePerms::new(true, true, false, true);
    let mut vas = [0u32; 3];
    for (idx, va) in vas.iter_mut().enumerate() {
        let region = first_region.saturating_add(L1_SPAN * idx as u32);
        *va = align_down_u32(region, PAGE_SIZE as u32);
        if !page_allocator::map_range_for_root(user_root, *va, PAGE_SIZE, perms) {
            return Err(34);
        }
    }

    log!("subtest: write data and verify translations across each region");
    for (idx, va) in vas.iter().enumerate() {
        let phys = page_allocator::translate(user_root, *va).unwrap_or(0);
        if phys == 0 {
            return Err(35);
        }
        let data = [0x90u8 + idx as u8, 0x91 + idx as u8, 0x92 + idx as u8, 0x93 + idx as u8];
        if !page_allocator::copy_user(user_root, *va, &data) {
            return Err(36);
        }
        let word = page_allocator::peek_word(user_root, *va).unwrap_or(0);
        let expected = (0x93u32 + idx as u32) << 24
            | (0x92u32 + idx as u32) << 16
            | (0x91u32 + idx as u32) << 8
            | (0x90u32 + idx as u32);
        if word != expected {
            return Err(37);
        }
    }
    Ok(())
}

fn test_map_to_physical_alignment_and_alias(
    user_root: u32,
    info: BootInfo,
) -> Result<(), u32> {
    // Description: map_to_physical must reject unaligned physical addresses and alias
    // when aligned.
    log!("test: map_to_physical alignment + aliasing");
    log!("subtest: create a source mapping to obtain a physical page");

    let source_va = align_down_u32(pick_user_va(info, 0x12000), PAGE_SIZE as u32);
    let perms = PagePerms::new(true, true, false, true);
    if !page_allocator::map_range_for_root(user_root, source_va, PAGE_SIZE, perms) {
        return Err(40);
    }
    let source_phys = page_allocator::translate(user_root, source_va).unwrap_or(0);
    if source_phys == 0 || source_phys % PAGE_SIZE != 0 {
        return Err(41);
    }

    log!("subtest: unaligned map_to_physical is rejected");
    let target_va = align_down_u32(pick_user_va(info, 0x18000), PAGE_SIZE as u32);
    if page_allocator::map_physical_range_for_root(
        user_root,
        target_va,
        (source_phys as u32).wrapping_add(1),
        PAGE_SIZE,
        perms,
    ) {
        return Err(42);
    }
    if page_allocator::translate(user_root, target_va).is_some() {
        return Err(43);
    }

    log!("subtest: aligned map_to_physical creates an alias");
    if !page_allocator::map_physical_range_for_root(
        user_root,
        target_va,
        source_phys as u32,
        PAGE_SIZE,
        perms,
    ) {
        return Err(44);
    }
    let aliased = page_allocator::translate(user_root, target_va).unwrap_or(0);
    if aliased != source_phys {
        return Err(45);
    }

    let data = [0x0cu8, 0x0d, 0x0e, 0x0f];
    if !page_allocator::copy_user(user_root, target_va, &data) {
        return Err(46);
    }
    let word = page_allocator::peek_word(user_root, source_va).unwrap_or(0);
    if word != 0x0f0e_0d0c {
        return Err(47);
    }
    Ok(())
}

fn test_mirror_gap_behavior(user_root: u32, info: BootInfo) -> Result<(), u32> {
    // Description: mirroring a range with an unmapped page should fail and only mirror
    // pages visited before the gap.
    log!("test: mirror range with a gap");
    log!("subtest: create two mapped pages with an unmapped gap");

    let base = align_down_u32(pick_user_va(info, 0x1e000), PAGE_SIZE as u32);
    let perms = PagePerms::new(true, true, false, true);
    if !page_allocator::map_range_for_root(user_root, base, PAGE_SIZE, perms) {
        return Err(50);
    }
    if !page_allocator::map_range_for_root(
        user_root,
        base.wrapping_add((PAGE_SIZE * 2) as u32),
        PAGE_SIZE,
        perms,
    ) {
        return Err(51);
    }

    log!("subtest: mirror across the gap and verify partial mirroring");
    let kernel_root = page_allocator::current_root();
    let gap_va = base.wrapping_add(PAGE_SIZE as u32);
    let end_va = base.wrapping_add((PAGE_SIZE * 2) as u32);
    let kernel_before_gap = page_allocator::translate(kernel_root, gap_va);
    let kernel_before_end = page_allocator::translate(kernel_root, end_va);
    let mirror_ok = page_allocator::mirror_user_range_into_kernel(
        user_root,
        base,
        PAGE_SIZE * 3,
        perms,
    );
    if mirror_ok {
        return Err(52);
    }
    let user_phys = page_allocator::translate(user_root, base).unwrap_or(0);
    let kernel_phys = page_allocator::translate(kernel_root, base).unwrap_or(0);
    if user_phys == 0 || kernel_phys != user_phys {
        return Err(53);
    }
    if page_allocator::translate(user_root, gap_va).is_some() {
        return Err(54);
    }
    let kernel_after_gap = page_allocator::translate(kernel_root, gap_va);
    if kernel_after_gap != kernel_before_gap {
        return Err(55);
    }
    let kernel_after_end = page_allocator::translate(kernel_root, end_va);
    if kernel_after_end != kernel_before_end {
        return Err(56);
    }
    Ok(())
}

fn test_copy_user_atomic(user_root: u32, info: BootInfo) -> Result<(), u32> {
    // Description: copy_user should be atomic (all-or-nothing) across page boundaries.
    log!("test: copy_user atomicity");
    log!("subtest: create writable + read-only pages");

    let base = align_down_u32(pick_user_va(info, 0x26000), PAGE_SIZE as u32);
    let perms_rw = PagePerms::new(true, true, false, true);
    let perms_ro = PagePerms::new(true, false, false, true);
    if !page_allocator::map_range_for_root(user_root, base, PAGE_SIZE, perms_rw) {
        return Err(60);
    }
    if !page_allocator::map_range_for_root(
        user_root,
        base.wrapping_add(PAGE_SIZE as u32),
        PAGE_SIZE,
        perms_ro,
    ) {
        return Err(61);
    }

    log!("subtest: seed data on both pages and near the boundary");
    let seed_first = [0x11u8, 0x22, 0x33, 0x44];
    let seed_second = [0xaau8, 0xbb, 0xcc, 0xdd];
    if !page_allocator::copy(user_root, base, &seed_first) {
        return Err(62);
    }
    if !page_allocator::copy(
        user_root,
        base.wrapping_add(PAGE_SIZE as u32),
        &seed_second,
    ) {
        return Err(63);
    }
    let boundary_seed = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let boundary_start = base.wrapping_add(PAGE_SIZE as u32 - 8);
    if !page_allocator::copy(user_root, boundary_start, &boundary_seed) {
        return Err(64);
    }

    log!("subtest: attempt a cross-page write and confirm no bytes changed");
    let mut big = [0u8; 16];
    for (i, byte) in big.iter_mut().enumerate() {
        *byte = (0x80 + i as u8) as u8;
    }
    let cross_start = boundary_start;
    if page_allocator::copy_user(user_root, cross_start, &big) {
        return Err(65);
    }
    let first_word = page_allocator::peek_word(user_root, base).unwrap_or(0);
    let boundary_word0 = page_allocator::peek_word(user_root, boundary_start).unwrap_or(0);
    let boundary_word1 = page_allocator::peek_word(
        user_root,
        boundary_start.wrapping_add(4),
    )
    .unwrap_or(0);
    let second_word = page_allocator::peek_word(
        user_root,
        base.wrapping_add(PAGE_SIZE as u32),
    )
    .unwrap_or(0);
    if first_word != 0x4433_2211
        || boundary_word0 != 0x0403_0201
        || boundary_word1 != 0x0807_0605
        || second_word != 0xddcc_bbaa
    {
        return Err(66);
    }
    Ok(())
}

fn test_remap_override_perms(user_root: u32, info: BootInfo) -> Result<(), u32> {
    // Description: remapping an existing VA should override permissions.
    log!("test: remap overrides permissions");
    log!("subtest: map RW, write data, then remap RO");

    let va = align_down_u32(pick_user_va(info, 0x2e000), PAGE_SIZE as u32);
    let perms_rw = PagePerms::new(true, true, false, true);
    let perms_ro = PagePerms::new(true, false, false, true);
    if !page_allocator::map_range_for_root(user_root, va, PAGE_SIZE, perms_rw) {
        return Err(70);
    }
    let first = [0x0au8, 0x0b, 0x0c, 0x0d];
    if !page_allocator::copy_user(user_root, va, &first) {
        return Err(71);
    }
    if !page_allocator::map_range_for_root(user_root, va, PAGE_SIZE, perms_ro) {
        return Err(72);
    }

    log!("subtest: verify write is rejected and existing data remains");
    let second = [0xf1u8, 0xf2, 0xf3, 0xf4];
    if page_allocator::copy_user(user_root, va, &second) {
        return Err(73);
    }
    let word = page_allocator::peek_word(user_root, va).unwrap_or(0);
    if word != 0x0d0c_0b0a {
        return Err(74);
    }
    Ok(())
}

fn pick_user_va(info: BootInfo, offset: u32) -> u32 {
    let window_start = info.va_base;
    let window_end = info.va_base.saturating_add(info.va_len);
    let mut candidate = window_start.saturating_add(offset);
    let len = PAGE_SIZE as u32 * 4;
    if candidate.saturating_add(len) > window_end {
        candidate = window_start.saturating_add(PAGE_SIZE as u32);
    }
    candidate
}

const fn align_down_u32(val: u32, align: u32) -> u32 {
    val & !(align - 1)
}

const fn align_up_u32(val: u32, align: u32) -> u32 {
    (val + (align - 1)) & !(align - 1)
}
