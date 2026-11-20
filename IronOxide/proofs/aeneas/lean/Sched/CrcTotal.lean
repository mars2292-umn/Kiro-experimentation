import Sched.ConfigTotal
open Aeneas Aeneas.Std Result

/-! # The checksums of the configuration and the layout return (2026-11-20)

`config_crc` and `layout_crc` (the boot's comparison of the tables with the
values the Generator emitted) read every table inside its length and
compute with shifts, masks and exclusive-or only. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NP : Usize}

set_option maxHeartbeats 8000000 in
@[step] theorem crc_new_t  :
    arch.bound.Crc32.new  ⦃ _ => True ⦄ := by
  unfold arch.bound.Crc32.new
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

set_option maxHeartbeats 8000000 in
@[step] theorem crc_byte_loop_t (crc : U32) (i : Usize) :
    arch.bound.Crc32.byte_loop crc i ⦃ _ => True ⦄ := by
  unfold arch.bound.Crc32.byte_loop
  apply loop.spec_decr_nat (fun (x : U32 × Usize) => 8 - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨crc1', i1'⟩ := x
    simp only
    unfold arch.bound.Crc32.byte_loop.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem crc_byte_t (self : arch.bound.Crc32) (b : U8) :
    arch.bound.Crc32.byte self b ⦃ _ => True ⦄ := by
  unfold arch.bound.Crc32.byte
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

set_option maxHeartbeats 8000000 in
@[step] theorem crc_u32_t (self : arch.bound.Crc32) (v : U32) :
    arch.bound.Crc32.u32 self v ⦃ _ => True ⦄ := by
  unfold arch.bound.Crc32.u32
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

set_option maxHeartbeats 8000000 in
@[step] theorem crc_u64_t (self : arch.bound.Crc32) (v : U64) :
    arch.bound.Crc32.u64 self v ⦃ _ => True ⦄ := by
  unfold arch.bound.Crc32.u64
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

set_option maxHeartbeats 8000000 in
@[step] theorem crc_finish_t (self : arch.bound.Crc32) :
    arch.bound.Crc32.finish self ⦃ _ => True ⦄ := by
  unfold arch.bound.Crc32.finish
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_loop0_t (cfg : logic.Config NT NR NP) (c : arch.bound.Crc32) (t : Usize) :
    arch.bound.config_crc_loop0 cfg c t ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc_loop0
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => NT.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', t1'⟩ := x
    simp only
    unfold arch.bound.config_crc_loop0.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_loop1_t (cfg : logic.Config NT NR NP) (c : arch.bound.Crc32) (r : Usize) :
    arch.bound.config_crc_loop1 cfg c r ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc_loop1
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => NR.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', r1'⟩ := x
    simp only
    unfold arch.bound.config_crc_loop1.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_loop2_loop0_t (cfg : logic.Config NT NR NP) (c : arch.bound.Crc32) (t : Usize) (r : Usize)
    (ht : t.val < NT.val) :
    arch.bound.config_crc_loop2_loop0 cfg c t r ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc_loop2_loop0
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => NR.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', r1'⟩ := x
    simp only
    unfold arch.bound.config_crc_loop2_loop0.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_loop2_t (cfg : logic.Config NT NR NP) (c : arch.bound.Crc32) (t : Usize) :
    arch.bound.config_crc_loop2 cfg c t ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc_loop2
  apply loop.spec_decr_nat (fun (x : logic.Config NT NR NP × arch.bound.Crc32 × Usize) => NT.val - x.2.2.val) (fun _ => True)
  · intro x _
    obtain ⟨cfg1', c1', t1'⟩ := x
    simp only
    unfold arch.bound.config_crc_loop2.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_loop3_t (cfg : logic.Config NT NR NP) (c : arch.bound.Crc32) (p : Usize) :
    arch.bound.config_crc_loop3 cfg c p ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc_loop3
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => NP.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', p1'⟩ := x
    simp only
    unfold arch.bound.config_crc_loop3.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_loop4_t (a : Array U8 NT) (c : arch.bound.Crc32) (t : Usize) :
    arch.bound.config_crc_loop4 a c t ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc_loop4
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => NT.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', t1'⟩ := x
    simp only
    unfold arch.bound.config_crc_loop4.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_loop5_t (a : Array U8 NT) (c : arch.bound.Crc32) (t : Usize) :
    arch.bound.config_crc_loop5 a c t ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc_loop5
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => NT.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', t1'⟩ := x
    simp only
    unfold arch.bound.config_crc_loop5.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_loop6_t (a : Array U8 NT) (c : arch.bound.Crc32) (t : Usize) :
    arch.bound.config_crc_loop6 a c t ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc_loop6
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => NT.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', t1'⟩ := x
    simp only
    unfold arch.bound.config_crc_loop6.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem config_crc_t (cfg : logic.Config NT NR NP) :
    arch.bound.config_crc cfg ⦃ _ => True ⦄ := by
  unfold arch.bound.config_crc
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

set_option maxHeartbeats 8000000 in
@[step] theorem layout_crc_loop0_t (a : Array logic.mpu.Interval 8#usize) (c : arch.bound.Crc32) (k : Usize) :
    arch.bound.layout_crc_loop0 a c k ⦃ _ => True ⦄ := by
  unfold arch.bound.layout_crc_loop0
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => 8 - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', k1'⟩ := x
    simp only
    unfold arch.bound.layout_crc_loop0.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem layout_crc_loop1_loop0_t (c : arch.bound.Crc32) (a : Array logic.mpu.Interval 5#usize) (a1 : Array Bool 5#usize) (w : Usize) :
    arch.bound.layout_crc_loop1_loop0 c a a1 w ⦃ _ => True ⦄ := by
  unfold arch.bound.layout_crc_loop1_loop0
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => 5 - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', w1'⟩ := x
    simp only
    unfold arch.bound.layout_crc_loop1_loop0.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem layout_crc_loop1_t (a : Array logic.mpu.PartitionLayout NP) (c : arch.bound.Crc32) (p : Usize) :
    arch.bound.layout_crc_loop1 a c p ⦃ _ => True ⦄ := by
  unfold arch.bound.layout_crc_loop1
  apply loop.spec_decr_nat (fun (x : arch.bound.Crc32 × Usize) => NP.val - x.2.val) (fun _ => True)
  · intro x _
    obtain ⟨c1', p1'⟩ := x
    simp only
    unfold arch.bound.layout_crc_loop1.body
    try simp only [max_windows, max_kwindows]
    ctac
    all_goals first
      | (scalar_tac; done)
      | (refine ⟨trivial, ?_⟩; scalar_tac; done)
      | (simp_all; scalar_tac)
  · trivial

set_option maxHeartbeats 8000000 in
@[step] theorem layout_crc_t (layout : logic.mpu.Layout NP) :
    arch.bound.layout_crc layout ⦃ _ => True ⦄ := by
  unfold arch.bound.layout_crc
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; scalar_tac)

end P18
