# Kernel crate

This crate implements the guest kernel that runs inside the VM. It is
responsible for bootstrapping the system, managing address spaces, launching
user programs, handling traps and syscalls, and returning receipts/results to
the bootloader.

## High-level responsibilities

- Boot-time initialization from `BootInfo` (page tables, heap, state).
- Page allocation and page-table mapping.
- Task creation and scheduling (single-threaded, cooperative).
- Trap entry/exit and syscall dispatch.
- User program loading, execution, and result collection.
- Kernel storage and receipt management.

## Boot and initialization flow

1) Bootloader writes `BootInfo` into guest memory and jumps to kernel entry.
2) `init_kernel` reads `BootInfo`, sets `ROOT_PPN`, initializes allocator
   state, and sets up the kernel heap window.
3) Trap vector is installed and the kernel starts processing transaction
   bundles. Each bundle is decoded and executed by creating tasks.

Key files:
- `src/init.rs`: kernel init, boot info parsing.
- `src/memory/`: allocator and page table helpers.
- `src/trap/`: trap entry and syscall dispatch.
- `src/task/`: task creation, scheduling, and context switch.

## Task model

A task represents an isolated user program execution with its own:
- Address space (root page table, ASID).
- Trapframe (PC, SP, argument registers).
- User heap pointer.
- Caller info (task id).

Core types:
- `Task` (`src/task/task.rs`): task state and context.
- `AddressSpace` (`src/task/task.rs`): root PPN + user VA window info.

### Task lifecycle

1) **Creation**
   - A fresh ASID and root page table are allocated.
   - The user program window is mapped with user permissions.
   - A dedicated call-args page is mapped just above the user window.
   - Program bytes are copied into the user window.
   - Call arguments (to/from/input) are copied into the call-args page.
   - The trapframe is initialized (PC, SP, A0..A3).

2) **Run**
   - `run_task` switches `satp` to the task root and jumps to user PC.
   - Syscalls trap back into the kernel via the trampoline page.

3) **Syscall handling**
   - The trap handler switches to kernel context, decodes syscall number,
     and dispatches to the appropriate handler in `src/syscall/`.
   - Results are written into the task context and returned to user.

4) **Completion**
   - The user program exits or triggers a breakpoint return.
   - The kernel collects the result payload and writes a receipt.
   - The kernel switches back to the caller task or the kernel task.

Task lifetime diagram:

```
   +---------+      +-----------+      +-------------+      +------------+
   | created | ---> | runnable  | ---> | running     | ---> | completed  |
   +---------+      +-----------+      +-------------+      +------------+
                          ^                    |
                          |                    v
                          +-------------- syscall/trap ----------+
```

## Memory layout

The kernel uses a flat VA window (from `BootInfo`) to access its heap and
static memory. User programs run in a separate user VA window starting at 0.

### User program VA window

Defined in `src/global.rs`:

- `PROGRAM_VA_BASE`: base of user mappings (0x0).
- `PROGRAM_WINDOW_BYTES`: total mapped user window size.
- `CODE_SIZE_LIMIT`: max code size.
- `RO_DATA_SIZE_LIMIT`: reserved rodata size.
- `HEAP_BYTES`: user heap size.
- `STACK_BYTES`: user stack size.
- `HEAP_START_ADDR`: heap base within the user window.

The stack is placed at the end of the user window and grows downward:

```
low VA                                                     high VA
| code | rodata | heap .............. | stack (grows down) |
^ PROGRAM_VA_BASE                     ^ stack_base         ^ stack_top
```

### Call-args page

Call arguments (to/from addresses and input buffer) live in a dedicated page
mapped just above the user window:

```
CALL_ARGS_PAGE_BASE = PROGRAM_VA_BASE + PROGRAM_WINDOW_BYTES
TO_PTR_ADDR         = CALL_ARGS_PAGE_BASE + 0x100
FROM_PTR_ADDR       = TO_PTR_ADDR + ADDRESS_LEN
INPUT_BASE_ADDR     = FROM_PTR_ADDR + ADDRESS_LEN
```

This keeps call-args separate from user heap/stack and avoids corruption.
The call-args page is mapped as user-read only.

User memory map (not to scale):

```
VA 0x00000000
  | code | rodata | heap .............. | stack |  call-args page |
  ^ PROGRAM_VA_BASE                     ^ stack_top              ^ CALL_ARGS_PAGE_BASE
VA 0x00000000 + PROGRAM_WINDOW_BYTES    (end of user window)
```

### Kernel heap

The kernel heap is a bump allocator initialized from `BootInfo.heap_ptr` and
bounded by the kernel VA window (`va_base .. va_base + va_len`). If the kernel
heap exhausts, the allocator panics.

## Trap and syscall flow

1) User executes an `ecall` or trap instruction.
2) Control transfers to the trampoline page (`TRAP_TRAMPOLINE_VA`).
3) Kernel trap handler saves user state and dispatches syscalls.
4) Syscall handlers live in `src/syscall/` and may read user memory using
   the task root page table.
5) Return values are placed in the trapframe and execution resumes in user.

Trap cycle diagram:

```
 user code
    |
    | ecall / fault
    v
 TRAP_TRAMPOLINE_VA  --->  kernel trap handler  --->  syscall handler
    ^                                                |
    |                                                v
 return to user  <-----------  update trapframe  <---+
```

## Storage and receipts

Kernel storage is maintained in the global `State` object. Syscalls can read
and write key/value pairs. Transaction receipts are written as tasks complete
and returned to the bootloader.

## Debugging notes

- Stack/heap overlap bugs are common if `stack_top` is not placed at the end
  of the user window.
- Call-args memory should be isolated from user heap/stack.
- Kernel heap exhaustion will panic in `src/memory/heap.rs`.

## Relevant files

- `src/global.rs`: global constants and kernel-wide state.
- `src/task/prep.rs`: program loading and trapframe setup.
- `src/task/run.rs`: context switch and run loop.
- `src/trap/mod.rs`: trap entry/exit and syscall dispatch.
- `src/syscall/`: syscall implementations.
- `src/memory/page_allocator.rs`: page allocation and mapping.
