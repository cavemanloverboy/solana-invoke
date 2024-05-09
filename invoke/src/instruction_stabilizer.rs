#![allow(unused)] // unused when target_os is not solana

use std::{marker::PhantomData, mem::ManuallyDrop, ptr::NonNull};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    stable_layout::stable_instruction::StableInstruction,
};

/// This wrapper type with no constructor ensures that no user can
/// manually drop the inner type.
///
/// We provide only an immutable borrow method, which ensures that
/// the inner type is not modified in the absence of unsafe code.
///
/// StableInstruction uses NonNull<T> which is invariant over T.
/// NonNull<T> is clonable. It's the same type used by Rc<T> and
/// Arc<T>. It is safe to have an aliasing p
/// ointer to the same
/// allocation as the underlying vectors so long as we perform
/// no modificiations.
///
/// A constructor api is chosen internally over simply making the inner type
/// pub(super) or pub(super) so that not even users within this crate (outside
/// of this module) can modify the inner type.
///
/// Most importantly...
/// No unsafe code was written in the creation or usage of this type :)
pub struct InstructionStabilizer<'a> {
    /// A stable instruction that will not be dropped. By circumventing the
    /// `Drop` implementation, this becomes a view (similar to a slice)
    /// into the original vector's buffer. Since we provide only a borrow
    /// method on this wrapper, we can guarantee that the `StableInstruction`
    /// is never modified.
    stabilized_instruction: core::mem::ManuallyDrop<StableInstruction>,

    /// A read-only view (into the buffers owned by the inner vectors) is
    /// only safe for as long as the `&'a Instruction` lives.
    ///
    /// This could be a `&'a Instruction` but we don't actually need the
    /// instruction. We can pretend to hold a `&'a Instruction`` instead.
    ///
    /// Using a `PhantomData<&'a Instruction>` forces this struct and the
    /// compiler to act like it is holding the reference without increasing
    /// the size of the type.
    phantom_instruction: PhantomData<&'a Instruction>,
}

impl<'ix> InstructionStabilizer<'ix> {
    #[inline(always)]
    pub fn stabilize(instruction: &Instruction) -> InstructionStabilizer {
        stabilize_instruction(instruction)
    }

    /// NOTE:
    ///
    /// A constructor api is chosen internally over simply making the inner type
    /// pub(super) or pub(super) so that not even users within this crate (outside
    /// of this module) can modify the inner type.
    #[inline(always)]
    pub(super) fn new(
        stabilized_instruction: core::mem::ManuallyDrop<StableInstruction>,
        // Note: This is where 'ix is inherited
        _instruction: &'ix Instruction,
    ) -> InstructionStabilizer<'ix> {
        Self {
            stabilized_instruction,
            phantom_instruction: PhantomData::<&'ix Instruction>,
        }
    }

    #[inline(always)]
    pub fn stable_instruction_ref<'borrow>(&'borrow self) -> &'borrow StableInstruction
    where
        // 'ix must live at least as long as 'borrow
        'ix: 'borrow,
    {
        &self.stabilized_instruction
    }

    #[inline(always)]
    pub fn instruction_addr(&self) -> *const u8 {
        self.stable_instruction_ref() as *const StableInstruction as *const u8
    }
}

#[repr(C)]
pub struct StableVec<T> {
    pub ptr: NonNull<T>,
    pub cap: usize,
    pub len: usize,
    _marker: PhantomData<T>,
}

// Only to be used by super::stable_instruction, but only ancestors are allowed for visibility
#[inline(always)] // only one call site (wrapper fn) so inline there
pub(super) fn stabilize_instruction<'ix_ref>(
    ix: &'ix_ref Instruction,
) -> InstructionStabilizer<'ix_ref> {
    // Get StableVec out of instruction data Vec<u8>
    let data: StableVec<u8> = {
        // Get vector parts
        let ptr = NonNull::new(ix.data.as_ptr() as *mut u8).expect("vector ptr should be valid");
        let len = ix.data.len();
        let cap = ix.data.capacity();

        StableVec {
            ptr,
            cap,
            len,
            _marker: std::marker::PhantomData,
        }
    };

    // Get StableVec out of instruction accounts Vec<Accountmeta>
    let accounts: StableVec<AccountMeta> = {
        // Get vector parts
        let ptr = NonNull::new(ix.accounts.as_ptr() as *mut AccountMeta)
            .expect("vector ptr should be valid");
        let len = ix.accounts.len();
        let cap = ix.accounts.capacity();

        StableVec {
            ptr,
            cap,
            len,
            _marker: std::marker::PhantomData,
        }
    };

    InstructionStabilizer::<'ix_ref>::new(
        ManuallyDrop::new(StableInstruction {
            // Transmuting between identically declared repr(C) structs
            accounts: unsafe { core::mem::transmute(accounts) },
            data: unsafe { core::mem::transmute(data) },
            program_id: ix.program_id,
        }),
        ix,
    )
}
