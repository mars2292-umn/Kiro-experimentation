import Sched.StubRefine

/-! # No exception between two instructions of a stub (2026-11-26)

The stub model treats each path of a stub as one sequence: no exception is
taken between two of its instructions. This file proves what that rests
on, of the machine of `Sched/Arm.lean`.

- `execPriority`, `preempts` (B1.5.4): an exception of priority `p` is
  taken only when `p` is below the execution priority, which PRIMASK
  raises to 0 and FAULTMASK to -1. `masked_no_configurable`: with either
  mask set no exception of configurable priority (0 or more) is taken;
  what can still be taken is a HardFault or an NMI, which end in the
  safe state and are not the stub model's.
- `execI_masks`, and the three `cps` lemmas: an instruction other than
  `cps` leaves both masks; `cpsid i` sets PRIMASK, `cpsid f` sets
  FAULTMASK (no HardFault or NMI active), `cpsie i` leaves FAULTMASK.
- `AlongRun`: a property of every state of a run. `Sched/StubPaths.lean`
  proves it of each path: the dispatch stub and `rsk_unwind` are masked at
  every instruction after their first (`cpsid i`) up to and including the
  exception return, and the Kernel-level stubs leave both masks as they
  found them, so their execution priority is that of their entry.

The first instruction of the dispatch stub is reached unmasked: that is
the entry-instruction window, which the model treats separately
(`nested_irq_resumed`, Verification_Plan 5.8a). -/

namespace Stub
open Arm

/-- The execution priority (B1.5.4): the priority `act` the active
exceptions and BASEPRI give, raised to 0 by PRIMASK and to -1 by
FAULTMASK. A lower number is a higher priority. -/
def execPriority (act : Int) (a : Arm.St) : Int :=
  min (min act (if a.primask then 0 else act)) (if a.faultmask then -1 else act)

/-- An exception of priority `p` is taken in state `a`. -/
def preempts (p act : Int) (a : Arm.St) : Prop := p < execPriority act a

/-- Either mask is set. -/
def Msk (a : Arm.St) : Prop := a.primask = true ∨ a.faultmask = true

/-- **With a mask set, no exception of configurable priority is taken.** -/
theorem masked_no_configurable {a : Arm.St} (h : Msk a) (p act : Int) (hp : 0 ≤ p) :
    ¬ preempts p act a := by
  unfold preempts execPriority
  rcases h with h | h <;> rw [h] <;> simp <;> omega

/-- With both masks as they were, the execution priority is as it was. -/
theorem execPriority_congr {a b : Arm.St} (h1 : b.primask = a.primask)
    (h2 : b.faultmask = a.faultmask) (act : Int) : execPriority act b = execPriority act a := by
  unfold execPriority
  rw [h1, h2]

theorem setR_primask (a : Arm.St) (n : Nat) (v : W) : (Arm.setR a n v).primask = a.primask := by
  unfold Arm.setR; split_ifs <;> rfl

theorem setR_faultmask (a : Arm.St) (n : Nat) (v : W) : (Arm.setR a n v).faultmask = a.faultmask := by
  unfold Arm.setR; split_ifs <;> rfl

/-- An instruction other than `cps` leaves PRIMASK and FAULTMASK. -/
theorem eff_masks {i : Instr} {len : W} {a a' : Arm.St} (h : eff i len a = some a')
    (hn : ∀ d fi ff, i ≠ .cps d fi ff) : a'.primask = a.primask ∧ a'.faultmask = a.faultmask := by
  cases i with
  | cps d fi ff => exact absurd rfl (hn d fi ff)
  | it _ _ => simp [eff] at h
  | bcond _ _ => simp [eff] at h
  | push regs =>
    simp only [eff] at h
    split at h
    · simp at h
    · simp only [Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | pop regs =>
    simp only [eff] at h
    split at h
    · simp at h
    · simp only [Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | vpush d regs =>
    simp only [eff, fpCheck] at h
    by_cases hl : a.lspact = true
    · simp [hl] at h
    · by_cases ha : a.aspen = true <;> simp [hl, ha] at h <;> obtain ⟨_, rfl⟩ := h <;>
        exact ⟨rfl, rfl⟩
  | vpop d regs =>
    simp only [eff, fpCheck] at h
    by_cases hl : a.lspact = true
    · simp [hl] at h
    · by_cases ha : a.aspen = true <;> simp [hl, ha] at h <;> obtain ⟨_, rfl⟩ := h <;>
        exact ⟨rfl, rfl⟩
  | mrs rd sysm =>
    simp only [eff] at h
    split_ifs at h <;> simp only [Option.map_some, Option.map_none, Option.some.injEq] at h <;>
      first
        | (subst h; exact ⟨setR_primask _ _ _, setR_faultmask _ _ _⟩)
        | (exact absurd h (by simp))
  | msr sysm rn =>
    simp only [eff] at h
    split_ifs at h <;> first
      | (simp only [Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩)
      | (simp at h)
  | movi rd imm =>
    simp only [eff] at h
    split_ifs at h <;> simp only [Option.some.injEq] at h <;> subst h <;>
      exact ⟨setR_primask _ _ _, setR_faultmask _ _ _⟩
  | movr rd rm =>
    simp only [eff, Option.some.injEq] at h; subst h
    exact ⟨setR_primask _ _ _, setR_faultmask _ _ _⟩
  | addrsp rd imm =>
    simp only [eff, Option.some.injEq] at h; subst h
    exact ⟨setR_primask _ _ _, setR_faultmask _ _ _⟩
  | ldrsp rt imm =>
    simp only [eff, Option.some.injEq] at h; subst h
    exact ⟨setR_primask _ _ _, setR_faultmask _ _ _⟩
  | tst rn imm => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | subsp imm => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | addsp imm => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | strsp rt imm => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | cmpi rn imm => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | dsb => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | isb => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | b off => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | cbz nz rn off => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | bl off => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | bx rm => simp only [eff, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩

/-- **An instruction other than `cps` leaves both masks**, executed or
skipped by its condition. -/
theorem execI_masks {i : Instr} {len : W} {a a' : Arm.St} (h : execI i len a = some a')
    (hn : ∀ d fi ff, i ≠ .cps d fi ff) : a'.primask = a.primask ∧ a'.faultmask = a.faultmask := by
  have key : ∀ (hi : ∀ fc mk, i ≠ .it fc mk) (hb : ∀ c o, i ≠ .bcond c o),
      a'.primask = a.primask ∧ a'.faultmask = a.faultmask := by
    intro hi hb
    have e : execI i len a =
        if condHolds (curCond a) a then (eff i len a).map fun t => { t with it := itAdvance a.it }
        else some { a with pc := a.pc + len, it := itAdvance a.it } := by
      cases i <;> first | rfl | exact absurd rfl (hi _ _) | exact absurd rfl (hb _ _)
    rw [e] at h
    split at h
    · cases he : eff i len a with
      | none => rw [he] at h; simp at h
      | some t =>
        rw [he] at h
        simp only [Option.map_some, Option.some.injEq] at h
        subst h
        have k := eff_masks (a' := t) he hn
        exact k
    · simp only [Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  cases i with
  | it fc mk => simp only [execI, Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | bcond c o =>
    simp only [execI] at h
    split at h
    · simp at h
    · simp only [Option.some.injEq] at h; subst h; exact ⟨rfl, rfl⟩
  | _ => exact key (by intros; simp) (by intros; simp)

/-- `cpsid i` sets PRIMASK and leaves FAULTMASK. -/
theorem execI_cpsid_i {len : W} {a a' : Arm.St} (hit : a.it = 0#8)
    (h : execI (.cps true true false) len a = some a') :
    a'.primask = true ∧ a'.faultmask = a.faultmask := by
  rw [execI_al hit (by simp) (by simp)] at h
  simp [eff] at h
  subst h
  simp

/-- `cpsid f`, no HardFault or NMI active, sets FAULTMASK and leaves PRIMASK. -/
theorem execI_cpsid_f {len : W} {a a' : Arm.St} (hit : a.it = 0#8) (hf : a.hfnmi = false)
    (h : execI (.cps true false true) len a = some a') :
    a'.faultmask = true ∧ a'.primask = a.primask := by
  rw [execI_al hit (by simp) (by simp)] at h
  simp [eff, hf] at h
  subst h
  simp

/-- `cpsie i` leaves FAULTMASK. -/
theorem execI_cpsie_i {len : W} {a a' : Arm.St} (hit : a.it = 0#8)
    (h : execI (.cps false true false) len a = some a') : a'.faultmask = a.faultmask := by
  rw [execI_al hit (by simp) (by simp)] at h
  simp [eff] at h
  subst h
  simp

/-- `P` holds of every state of the run of `n` steps from `a`, and the run
does not stop. -/
def AlongRun (code : W → Option Nat) (P : Arm.St → Prop) : Nat → Arm.St → Prop
  | 0, a => P a
  | n + 1, a => P a ∧ ∃ a', step code a = some a' ∧ AlongRun code P n a'

/-- What `AlongRun` says of the run. -/
theorem AlongRun.at {code : W → Option Nat} {P : Arm.St → Prop} :
    ∀ {n : Nat} {a : Arm.St}, AlongRun code P n a → ∀ k, k ≤ n → ∀ s, run code k a = some s → P s := by
  intro n
  induction n with
  | zero =>
    intro a h k hk s hs
    have : k = 0 := by omega
    subst this
    simp only [run, Option.some.injEq] at hs
    subst hs
    exact h
  | succ n ih =>
    intro a h k hk s hs
    obtain ⟨hp, a', hst, hrest⟩ := h
    cases k with
    | zero => simp only [run, Option.some.injEq] at hs; subst hs; exact hp
    | succ k =>
      simp only [run, hst, Option.bind_some] at hs
      exact ih hrest k (by omega) s hs

theorem AlongRun.last {code : W → Option Nat} {P : Arm.St → Prop} {a : Arm.St} (h : P a) :
    AlongRun code P 0 a := h

theorem AlongRun.cons {code : W → Option Nat} {P : Arm.St → Prop} {n : Nat} {a a' : Arm.St}
    (h : P a) (hs : step code a = some a') (hr : AlongRun code P n a') :
    AlongRun code P (n + 1) a := ⟨h, a', hs, hr⟩

/-- Both masks are those of `a`. -/
def EqM (a s : Arm.St) : Prop := s.primask = a.primask ∧ s.faultmask = a.faultmask

end Stub
