import Sched.BoundAll
open Aeneas Aeneas.Std Result

/-! # The buffer checks answer their definitions (2026-11-20)

`PartitionLayout::message_permitted_exec` and `dma_permitted_exec` are what
the Kernel runs before it copies a message for an Endpoint operation
(R42.2) and before it programs an EasyDMA transfer (R17.1, R17.2). Verus
proves each equal to its specification (`ensures ok ==
self.message_permitted(..)`, `ensures ok == self.dma_permitted(..)`); here
the same on the extracted code, with the specifications written after
`mpu.rs`. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

/-- `PartitionLayout::message_permitted`: the buffer is not empty and lies
in the Partition's RAM interval. -/
def MsgPermitted (pl : logic.mpu.PartitionLayout) (ptr len : Nat) : Prop :=
  0 < len ∧ pl.ram.base.val ≤ ptr ∧ ptr + len ≤ pl.ram.base.val + 2 ^ pl.ram.size_log2.val

/-- **`message_permitted_exec` returns `true` exactly for a permitted
buffer.** -/
theorem message_permitted_exec_v (pl : logic.mpu.PartitionLayout) (ptr len : U32)
    (h : pl.ram.size_log2.val ≤ 31) :
    logic.mpu.PartitionLayout.message_permitted_exec pl ptr len ⦃ r =>
      r = true ↔ MsgPermitted pl ptr.val len.val ⦄ := by
  unfold logic.mpu.PartitionLayout.message_permitted_exec
  ctac
  all_goals first
    | (scalar_tac; done)
    | (simp_all [MsgPermitted]; done)
    | (simp_all [MsgPermitted]; scalar_tac; done)
    | (simp_all [MsgPermitted]; omega)

/-- Subregion-disable bit `j` of an SRD byte (`srd_bit`). -/
def srdBit (srd : U8) (j : Nat) : Nat := (srd.val >>> j) % 2

/-- Subregion `j` of the RAM (an eighth, `sub` bytes from `base + j * sub`)
that the buffer `[p, p + l)` overlaps is enabled. -/
def SubOk (srd : U8) (base sub p l j : Nat) : Prop :=
  base + j * sub < p + l → p < base + (j + 1) * sub → srdBit srd j = 0

/-- `PartitionLayout::dma_permitted`: the buffer is not empty, lies in the
Partition's RAM above its stack region, and touches only enabled
subregions. -/
def DmaPermitted (pl : logic.mpu.PartitionLayout) (ptr len : Nat) : Prop :=
  0 < len ∧ pl.ram.base.val + pl.stack_size.val ≤ ptr ∧
  ptr + len ≤ pl.ram.base.val + 2 ^ pl.ram.size_log2.val ∧
  ∀ j, j < 8 → SubOk pl.ram_srd pl.ram.base.val (2 ^ pl.ram.size_log2.val / 8) ptr len j

set_option maxHeartbeats 8000000 in
theorem dma_loop_v (i : U8) (p l sub : U64) (s : U8) (ok1 : Bool) (lo : U64) (base : Nat)
    (hp : p.val < 4294967296) (hl : l.val < 4294967296) (hsub : sub.val ≤ 4294967296)
    (hbase : base < 4294967296) (hs : s.val ≤ 8) (hlo : lo.val = base + s.val * sub.val) :
    logic.mpu.PartitionLayout.dma_permitted_exec_loop i p l sub s ok1 lo ⦃ r =>
      r = true ↔ (ok1 = true ∧ ∀ j, s.val ≤ j → j < 8 → SubOk i base sub.val p.val l.val j) ⦄ := by
  unfold logic.mpu.PartitionLayout.dma_permitted_exec_loop
  apply loop.spec_decr_nat (fun (x : U8 × Bool × U64) => 8 - x.1.val)
    (fun x => x.1.val ≤ 8 ∧ x.2.2.val = base + x.1.val * sub.val ∧
      (x.2.1 = true ↔ (ok1 = true ∧ ∀ j, s.val ≤ j → j < x.1.val →
        SubOk i base sub.val p.val l.val j)) ∧ s.val ≤ x.1.val)
  · intro x hx
    obtain ⟨s1, o1, lo1⟩ := x
    simp only at hx ⊢
    obtain ⟨h8, hlo1, hiff, hss⟩ := hx
    unfold logic.mpu.PartitionLayout.dma_permitted_exec_loop.body
    split
    · rename_i hlt
      have hlt' : s1.val < 8 := by scalar_tac
      have hm : s1.val * sub.val ≤ 8 * sub.val := Nat.mul_le_mul_right _ (by omega)
      step as ⟨hi, hhi⟩
      step as ⟨i1, hi1⟩
      have hhi' : hi.val = base + (s1.val + 1) * sub.val := by rw [hhi, hlo1, Nat.succ_mul]; omega
      refine WP.spec_bind (Pₘ := fun hit : Bool =>
        hit = true ↔ (lo1.val < i1.val ∧ p.val < hi.val ∧ srdBit i s1.val = 1)) ?_ (fun hit hh => ?_)
      · split
        · split
          · step as ⟨i2, hi2⟩
            step as ⟨i3, hi3⟩
            have h1 : (1#u8).val = 1 := rfl
            have e3 : i3.val = srdBit i s1.val := by
              rw [hi3, UScalar.val_and, hi2, h1]
              exact Nat.and_one_is_mod _
            first | refine (WP.spec_ok _).mpr ?_ | skip
            constructor
            · intro hd
              have h3 : i3 = 1#u8 := by simpa using hd
              refine ⟨by scalar_tac, by scalar_tac, ?_⟩
              rw [← e3, h3]
              exact h1
            · rintro ⟨_, _, hb⟩
              have h3 : i3 = 1#u8 := by
                have : i3.val = 1 := by rw [e3]; exact hb
                scalar_tac
              simp [h3]
          · refine (WP.spec_ok _).mpr ⟨fun h => Bool.noConfusion h, fun h => ?_⟩
            exact absurd h.2.1 (by scalar_tac)
        · refine (WP.spec_ok _).mpr ⟨fun h => Bool.noConfusion h, fun h => ?_⟩
          exact absurd h.1 (by scalar_tac)
      · have hbit : srdBit i s1.val = 0 ∨ srdBit i s1.val = 1 := Nat.mod_two_eq_zero_or_one _
        have hsub : SubOk i base sub.val p.val l.val s1.val ↔ ¬ hit = true := by
          rw [hh]
          unfold SubOk
          rw [← hlo1, ← hhi', ← hi1]
          constructor
          · rintro hc ⟨a1, a2, a3⟩
            have := hc a1 a2
            omega
          · intro hn a1 a2
            rcases hbit with h0 | h1
            · exact h0
            · exact absurd ⟨a1, a2, h1⟩ hn
        have hall : ∀ o2 : Bool, (o2 = true ↔ (o1 = true ∧ ¬ hit = true)) →
            (o2 = true ↔ (ok1 = true ∧ ∀ j, s.val ≤ j → j < s1.val + 1 →
              SubOk i base sub.val p.val l.val j)) := by
          intro o2 ho2
          rw [ho2, hiff, ← hsub]
          constructor
          · rintro ⟨⟨h1, h2⟩, h3⟩
            refine ⟨h1, fun j hj hj' => ?_⟩
            rcases Nat.lt_succ_iff_lt_or_eq.mp hj' with h | h
            · exact h2 j hj h
            · rw [h]; exact h3
          · rintro ⟨h1, h2⟩
            exact ⟨⟨h1, fun j hj hj' => h2 j hj (by omega)⟩, h2 s1.val hss (by omega)⟩
        split
        · rename_i hht
          step as ⟨s2, hs2⟩
          refine ⟨by scalar_tac, by rw [hs2]; exact hhi', ?_, by scalar_tac, by scalar_tac⟩
          rw [hs2]
          exact hall false ⟨fun h => Bool.noConfusion h, fun h => absurd hht h.2⟩
        · rename_i hht
          step as ⟨s2, hs2⟩
          refine ⟨by scalar_tac, by rw [hs2]; exact hhi', ?_, by scalar_tac, by scalar_tac⟩
          rw [hs2]
          exact hall o1 ⟨fun h => ⟨h, hht⟩, fun h => h.1⟩
    · step*
  · refine ⟨hs, hlo, ⟨fun h => ⟨h, fun j h1 h2 => ?_⟩, fun h => h.1⟩, Nat.le_refl _⟩
    simp only at h2
    omega

/-- **`dma_permitted_exec` returns `true` exactly for a permitted buffer.** -/
theorem dma_permitted_exec_v (pl : logic.mpu.PartitionLayout) (ptr len : U32)
    (h : pl.ram.size_log2.val ≤ 31) :
    logic.mpu.PartitionLayout.dma_permitted_exec pl ptr len ⦃ r =>
      r = true ↔ DmaPermitted pl ptr.val len.val ⦄ := by
  unfold logic.mpu.PartitionLayout.dma_permitted_exec
  split
  · rename_i h0
    refine (WP.spec_ok _).mpr ⟨fun hf => Bool.noConfusion hf, fun hd => ?_⟩
    have := hd.1
    scalar_tac
  · rename_i h0
    step as ⟨i, hi, hipos⟩
    step as ⟨size, hsize⟩
    step as ⟨base, hbase⟩
    step as ⟨p, hp⟩
    step as ⟨l, hl⟩
    step as ⟨i1, hi1⟩
    step as ⟨i2, hi2⟩
    split
    · rename_i hlt
      refine (WP.spec_ok _).mpr ⟨fun hf => Bool.noConfusion hf, fun hd => ?_⟩
      have := hd.2.1
      scalar_tac
    · rename_i hge
      step as ⟨i3, hi3⟩
      step as ⟨i4, hi4⟩
      split
      · rename_i hgt
        refine (WP.spec_ok _).mpr ⟨fun hf => Bool.noConfusion hf, fun hd => ?_⟩
        have := hd.2.2.1
        scalar_tac
      · rename_i hle
        step as ⟨sub, hsub⟩
        have hsv : sub.val = 2 ^ pl.ram.size_log2.val / 8 := by scalar_tac
        refine WP.spec_mono (dma_loop_v pl.ram_srd p l sub 0#u8 true base pl.ram.base.val
          (by scalar_tac) (by scalar_tac) (by scalar_tac) (by scalar_tac) (by scalar_tac)
          (by scalar_tac)) (fun r hr => ?_)
        rw [hr]
        unfold DmaPermitted
        rw [← hsv]
        constructor
        · rintro ⟨_, hall⟩
          refine ⟨by scalar_tac, by scalar_tac, by scalar_tac, fun j hj => ?_⟩
          have := hall j (by scalar_tac) hj
          simpa [hp, hl] using this
        · rintro ⟨_, _, _, hall⟩
          refine ⟨rfl, fun j _ hj => ?_⟩
          have := hall j hj
          simpa [hp, hl] using this

end P18
