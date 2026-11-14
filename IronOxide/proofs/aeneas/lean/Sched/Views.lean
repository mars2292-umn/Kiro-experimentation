import Sched.Locks
open Aeneas Aeneas.Std Result

/-!
The scheduler state read as functions over the naturals, and each
extracted transition as a change of those functions.
-/

namespace P18
open rsk_logic

variable {NT NR NL NP NS NE : Usize}

def rel (s : logic.sched.Sched NT NR NL) (k : Nat) : Nat := (s.release.val[k]!).val
def job (s : logic.sched.Sched NT NR NL) (k : Nat) : Nat := (s.job.val[k]!).val
def stk (s : logic.sched.Sched NT NR NL) (p : Nat) : Nat := (s.stack.val[p]!).val

theorem get_set {α} [Inhabited α] {n : Usize} (v : Array α n) (i : Usize) (x : α) (k : Nat)
    (hi : i.val < n.val) :
    ((v.set i x).val[k]!) = if k = i.val then x else v.val[k]! := by
  have := v.property
  simp only [Array.set_val_eq]
  split
  · rename_i h; subst h; simp [*]
  · rename_i h; simp [*]

/-- The lock stack and the System_Ceiling are unchanged. -/
def LockSame (s' s : logic.sched.Sched NT NR NL) : Prop :=
  (∀ p, lres s' p = lres s p) ∧ (∀ p, ldep s' p = ldep s p) ∧ s'.lock_len = s.lock_len ∧
  s'.ceiling = s.ceiling

/-- The Release and Job states and the stack are unchanged. -/
def StackSame (s' s : logic.sched.Sched NT NR NL) : Prop :=
  (∀ k, rel s' k = rel s k) ∧ (∀ k, job s' k = job s k) ∧ (∀ p, stk s' p = stk s p) ∧
  s'.depth = s.depth

theorem make_pending_views {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.make_pending s cfg t = ok s') :
    t.val < NT.val ∧ (∀ k, rel s' k = if k = t.val then 1 else rel s k) ∧
    (∀ k, job s' k = job s k) ∧ (∀ p, stk s' p = stk s p) ∧ s'.depth = s.depth ∧
    LockSame s' s := by
  obtain ⟨ht, rfl⟩ := make_pending_ok h
  refine ⟨ht, ?_, fun _ => rfl, fun _ => rfl, rfl, fun _ => rfl, fun _ => rfl, rfl, rfl⟩
  intro k
  simp only [rel]
  rw [get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht), U8.cast_Usize_val_eq, rel_pending]
  split <;> rfl

theorem start_views {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {t : U8} (h : logic.sched.Sched.start s cfg t = ok s') :
    t.val < NT.val ∧ s.depth.val < NT.val ∧
    (∀ k, rel s' k = if k = t.val then 0 else rel s k) ∧
    (∀ k, job s' k = if k = t.val then 1 else job s k) ∧
    (∀ p, stk s' p = if p = s.depth.val then t.val else stk s p) ∧
    s'.depth.val = s.depth.val + 1 ∧ LockSame s' s := by
  obtain ⟨ht, hd, d, hd', rfl⟩ := start_ok h
  refine ⟨ht, hd, ?_, ?_, ?_, hd', fun _ => rfl, fun _ => rfl, rfl, rfl⟩
  · intro k
    simp only [rel]
    rw [get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht), U8.cast_Usize_val_eq, rel_none]
    split <;> rfl
  · intro k
    simp only [job]
    rw [get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact ht), U8.cast_Usize_val_eq, job_running]
    split <;> rfl
  · intro p
    simp only [stk]
    rw [get_set _ _ _ _ hd]
    split <;> rfl

theorem lock_views {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.lock s cfg r = ok (true, s')) :
    StackSame s' s ∧ 1 ≤ s.depth.val ∧ s.lock_len.val < NL.val ∧
    s'.lock_len.val = s.lock_len.val + 1 ∧
    (∀ p, lres s' p = if p = s.lock_len.val then r.val else lres s p) ∧
    (∀ p, ldep s' p = if p = s.lock_len.val then s.depth.val - 1 else ldep s p) ∧
    s'.ceiling.val = max s.ceiling.val (ceil cfg r.val) := by
  obtain ⟨hd, hl, n, d, c, hn, hdv, hc, rfl⟩ := lock_ok h
  refine ⟨⟨fun _ => rfl, fun _ => rfl, fun _ => rfl, rfl⟩, hd, hl, hn, ?_, ?_, hc⟩
  · intro p
    simp only [lres]
    rw [get_set _ _ _ _ hl]
    split <;> rfl
  · intro p
    simp only [ldep]
    rw [get_set _ _ _ _ hl]
    by_cases hp : p = s.lock_len.val
    · simp only [hp, ite_true]; exact hdv
    · simp only [hp, ite_false]

theorem unlock_views {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.unlock s cfg r = ok (true, s')) :
    StackSame s' s ∧ 1 ≤ s.lock_len.val ∧ 1 ≤ s.depth.val ∧
    lres s (s.lock_len.val - 1) = r.val ∧ ldep s (s.lock_len.val - 1) = s.depth.val - 1 ∧
    s'.lock_len.val = s.lock_len.val - 1 ∧
    (∀ p, lres s' p = lres s p) ∧ (∀ p, ldep s' p = ldep s p) ∧
    IsCeil cfg s (s.lock_len.val - 1) s'.ceiling.val := by
  obtain ⟨h1, h2, h3, h4, n, c, hn, hc, rfl⟩ := unlock_ok h
  refine ⟨⟨fun _ => rfl, fun _ => rfl, fun _ => rfl, rfl⟩, h1, h2, h3, h4, hn, fun _ => rfl,
    fun _ => rfl, ?_⟩
  rw [← hn]; exact hc

theorem finish_top_views {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r : Usize} (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) :
    1 ≤ s.depth.val ∧ s.depth.val ≤ NT.val ∧ stk s (s.depth.val - 1) < NT.val ∧
    (∀ k, rel s' k = rel s k) ∧
    (∀ k, job s' k = if k = stk s (s.depth.val - 1) then 0 else job s k) ∧
    (∀ p, stk s' p = stk s p) ∧ s'.depth.val = s.depth.val - 1 ∧
    (∀ p, lres s' p = lres s p) ∧ (∀ p, ldep s' p = ldep s p) ∧
    s'.lock_len.val ≤ s.lock_len.val ∧
    (∀ p, s'.lock_len.val ≤ p → p < s.lock_len.val → ldep s p = s.depth.val - 1) ∧
    (s'.lock_len.val = 0 ∨ ldep s (s'.lock_len.val - 1) ≠ s.depth.val - 1) ∧
    IsCeil cfg s s'.lock_len.val s'.ceiling.val := by
  obtain ⟨h1, h2, h3, d, n, c, hd, g1, g2, g3, hc, rfl⟩ := finish_top_ok h
  refine ⟨h1, h2, h3, fun _ => rfl, ?_, fun _ => rfl, hd, fun _ => rfl, fun _ => rfl, g1, g2, g3, hc⟩
  intro k
  simp only [job, stk]
  rw [get_set _ _ _ _ (by rw [U8.cast_Usize_val_eq]; exact h3), U8.cast_Usize_val_eq, job_none]
  by_cases hk : k = (s.stack.val[s.depth.val - 1]!).val
  · simp only [hk, ite_true]; rfl
  · simp only [hk, ite_false]

end P18
