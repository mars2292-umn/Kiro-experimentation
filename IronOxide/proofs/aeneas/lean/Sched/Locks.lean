import Sched.Model
open Aeneas Aeneas.Std Result

/-!
What the extracted `lock`, `unlock` and `end_job` return, whenever they
return, with their loops (stage 2 of Verification_Plan 7.3).
-/

namespace P18
open rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The Ceiling of Resource `r` in the Kernel's configuration (0 out of range). -/
def ceil (cfg : logic.Config NT NR NP) (r : Nat) : Nat :=
  match cfg.resources.val[r]? with
  | some rc => rc.ceiling.val
  | none => 0

/-- The Resource of the lock stack's entry `p`, and the stack position of
the Job that holds it. -/
def lres (s : logic.sched.Sched NT NR NL) (p : Nat) : Nat := (s.lock_res.val[p]!).val
def ldep (s : logic.sched.Sched NT NR NL) (p : Nat) : Nat := (s.lock_depth.val[p]!).val

/-- `end_job`'s loop pops the entries of the Job at stack position `d`
from the top of the lock stack. -/
theorem end_job_loop_ok (a : Array Usize NL) (d : Usize) :
    ∀ (k : Nat) (n n' : Usize), n.val = k → logic.sched.Sched.end_job_loop a d n = ok n' →
      n'.val ≤ n.val ∧ (∀ p, n'.val ≤ p → p < n.val → a.val[p]! = d) ∧
      (n'.val = 0 ∨ a.val[n'.val - 1]! ≠ d) := by
  intro k
  induction k with
  | zero =>
    intro n n' hn h
    have hn0 : n = 0#usize := by scalar_tac
    subst hn0
    rw [end_job_loop_zero] at h
    have h := ok_inj h
    subst h
    refine ⟨Nat.le_refl _, ?_, Or.inl rfl⟩
    intro p _ hp
    simp at hp
  | succ k ih =>
    intro n n' hn h
    unfold logic.sched.Sched.end_job_loop loop at h
    simp only [logic.sched.Sched.end_job_loop.body] at h
    have hpos : n > 0#usize := by scalar_tac
    simp only [hpos, ite_true, bind_eq_ok] at h
    obtain ⟨cf, ⟨m, hm, x, hx, hif⟩, hloop⟩ := h
    obtain ⟨_, hmv⟩ := usize_sub_ok hm
    obtain ⟨hmb, rfl⟩ := index_ok hx
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hmv
    by_cases hxd : a.val[m.val]! = d
    · simp only [hxd, ite_true] at hif
      have := ok_inj hif
      subst this
      have hrec : logic.sched.Sched.end_job_loop a d m = ok n' := by
        unfold logic.sched.Sched.end_job_loop
        simp only [logic.sched.Sched.end_job_loop.body]
        exact hloop
      obtain ⟨h1', h2', h3'⟩ := ih m n' (by omega) hrec
      refine ⟨by omega, ?_, h3'⟩
      intro p hp1 hp2
      by_cases hpm : p = m.val
      · rw [hpm]; exact hxd
      · exact h2' p hp1 (by omega)
    · simp only [hxd, ite_false] at hif
      have := ok_inj hif
      subst this
      have := ok_inj hloop
      subst this
      refine ⟨Nat.le_refl _, fun p h1 h2 => by omega, Or.inr ?_⟩
      rw [← hmv]; exact hxd

/-- `recompute_ceiling`'s loop returns the greatest of its start value and
the Ceilings of the entries it visits. -/
theorem recompute_loop_ok (s : logic.sched.Sched NT NR NL) (cfg : logic.Config NT NR NP)
    (n : Usize) :
    ∀ (k : Nat) (c : U8) (e : Usize) (c' : U8), n.val - e.val = k →
      logic.sched.Sched.recompute_ceiling_loop s cfg n c e = ok c' →
      c.val ≤ c'.val ∧ (∀ p, e.val ≤ p → p < n.val → ceil cfg (lres s p) ≤ c'.val) ∧
      (c' = c ∨ ∃ p, e.val ≤ p ∧ p < n.val ∧ c'.val = ceil cfg (lres s p)) := by
  intro k
  induction k with
  | zero =>
    intro c e c' hk h
    unfold logic.sched.Sched.recompute_ceiling_loop loop at h
    simp only [logic.sched.Sched.recompute_ceiling_loop.body] at h
    have hlt : ¬ e < n := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have := ok_inj h
    subst this
    exact ⟨Nat.le_refl _, fun p h1 h2 => by omega, Or.inl rfl⟩
  | succ k ih =>
    intro c e c' hk h
    unfold logic.sched.Sched.recompute_ceiling_loop loop at h
    simp only [logic.sched.Sched.recompute_ceiling_loop.body] at h
    have hlt : e < n := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨r, hr, rc, hrc, c1, hc1, e1, he1, hcf⟩, hloop⟩ := h
    obtain ⟨_, rfl⟩ := index_ok hr
    obtain ⟨_, hrc'⟩ := index_ok' hrc
    rw [U8.cast_Usize_val_eq] at hrc'
    have hceil : ceil cfg (lres s e.val) = rc.ceiling.val := by
      unfold ceil lres; rw [hrc']
    have he1v := usize_add_ok he1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at he1v
    have := ok_inj hcf
    subst this
    have hrec : logic.sched.Sched.recompute_ceiling_loop s cfg n c1 e1 = ok c' := by
      unfold logic.sched.Sched.recompute_ceiling_loop
      simp only [logic.sched.Sched.recompute_ceiling_loop.body, lift, bind_ok]
      exact hloop
    obtain ⟨g1, g2, g3⟩ := ih c1 e1 c' (by omega) hrec
    have hc1v : c.val ≤ c1.val ∧ rc.ceiling.val ≤ c1.val ∧ (c1 = c ∨ c1 = rc.ceiling) := by
      by_cases hgt : rc.ceiling > c
      · simp only [hgt, ite_true] at hc1
        have := ok_inj hc1
        subst this
        exact ⟨by scalar_tac, Nat.le_refl _, Or.inr rfl⟩
      · simp only [hgt, ite_false] at hc1
        have := ok_inj hc1
        subst this
        exact ⟨Nat.le_refl _, by scalar_tac, Or.inl rfl⟩
    refine ⟨by omega, ?_, ?_⟩
    · intro p hp1 hp2
      by_cases hpe : p = e.val
      · rw [hpe, hceil]; omega
      · exact g2 p (by omega) hp2
    · rcases g3 with g | ⟨p, hp1, hp2, hp3⟩
      · rcases hc1v.2.2 with h | h
        · left; rw [g, h]
        · right; exact ⟨e.val, Nat.le_refl _, by scalar_tac, by rw [g, h, hceil]⟩
      · right; exact ⟨p, by omega, hp2, hp3⟩

theorem index_mut_ok {α} {n : Usize} {v : Array α n} {i : Usize} {x : α} {back : α → Array α n}
    (h : v.index_mut_usize i = ok (x, back)) : i.val < n.val ∧ back = v.set i := by
  unfold Array.index_mut_usize at h
  simp only [bind_tc_eq, bind_eq_ok] at h
  obtain ⟨y, hy, h⟩ := h
  have := ok_inj h
  simp only [Prod.mk.injEq] at this
  exact ⟨(index_ok' hy).1, this.2.symm⟩

/-- A granted `lock`: the entry is pushed, held by the Job on top of the
stack, and the System_Ceiling is raised to the Resource's Ceiling if that
is higher. -/
theorem lock_ok {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.lock s cfg r = ok (true, s')) :
    1 ≤ s.depth.val ∧ s.lock_len.val < NL.val ∧
    ∃ (n d : Usize) (c : U8), n.val = s.lock_len.val + 1 ∧ d.val = s.depth.val - 1 ∧
      c.val = max s.ceiling.val (ceil cfg r.val) ∧
      s' = { s with
        ceiling := c, lock_res := s.lock_res.set s.lock_len r,
        lock_depth := s.lock_depth.set s.lock_len d, lock_len := n } := by
  simp only [logic.sched.Sched.lock, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨d, hd, t, ht, a, ha, b, hb, h⟩ := h
  obtain ⟨hd1, hd2⟩ := usize_sub_ok hd
  have h1 : (1#usize).val = 1 := rfl
  rw [h1] at hd1 hd2
  split at h
  · simp only [bind_eq_ok] at h
    obtain ⟨re, hre, h⟩ := h
    split at h
    · cases (Prod.mk.inj (ok_inj h)).1
    · split at h
      · cases (Prod.mk.inj (ok_inj h)).1
      · rename_i hlen
        simp only [bind_eq_ok] at h
        obtain ⟨rc, hrc, ⟨x1, back⟩, hback, h⟩ := h
        erw [bind_eq_ok] at h
        obtain ⟨⟨x2, back1⟩, hback1, h⟩ := h
        erw [bind_eq_ok] at h
        obtain ⟨n, hn, h⟩ := h
        obtain ⟨hl, rfl⟩ := index_mut_ok hback
        obtain ⟨_, rfl⟩ := index_mut_ok hback1
        have hnv := usize_add_ok hn
        rw [h1] at hnv
        obtain ⟨_, hrc'⟩ := index_ok' hrc
        rw [U8.cast_Usize_val_eq] at hrc'
        have hceil : ceil cfg r.val = rc.ceiling.val := by
          unfold ceil; rw [hrc']
        split at h
        · rename_i hgt
          have h := (Prod.mk.inj (ok_inj h)).2
          exact ⟨hd1, hl, n, d, rc.ceiling, hnv, hd2, by rw [hceil]; scalar_tac, h.symm⟩
        · rename_i hgt
          have h := (Prod.mk.inj (ok_inj h)).2
          exact ⟨hd1, hl, n, d, s.ceiling, hnv, hd2, by rw [hceil]; scalar_tac, h.symm⟩
  · cases (Prod.mk.inj (ok_inj h)).1

/-- `c` is the System_Ceiling of the lock stack's first `n` entries: the
greatest of their Resources' Ceilings, 0 for none. -/
def IsCeil (cfg : logic.Config NT NR NP) (s : logic.sched.Sched NT NR NL) (n c : Nat) : Prop :=
  (∀ p, p < n → ceil cfg (lres s p) ≤ c) ∧ (c = 0 ∨ ∃ p, p < n ∧ c = ceil cfg (lres s p))

theorem recompute_ok {s : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {n : Usize} {c : U8} (h : logic.sched.Sched.recompute_ceiling s cfg n = ok c) :
    IsCeil cfg s n.val c.val := by
  unfold logic.sched.Sched.recompute_ceiling at h
  obtain ⟨_, g2, g3⟩ := recompute_loop_ok s cfg n _ _ _ _ rfl h
  have h0 : (0#usize).val = 0 := rfl
  refine ⟨fun p hp => g2 p (by omega) hp, ?_⟩
  rcases g3 with g | ⟨p, _, hp, hc⟩
  · left; rw [g, idle_ceiling]; rfl
  · right; exact ⟨p, hp, hc⟩

/-- A granted `unlock`: the top entry, held by the Job on top of the stack,
is popped and the System_Ceiling recomputed. -/
theorem unlock_ok {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.unlock s cfg r = ok (true, s')) :
    1 ≤ s.lock_len.val ∧ 1 ≤ s.depth.val ∧ lres s (s.lock_len.val - 1) = r.val ∧
    ldep s (s.lock_len.val - 1) = s.depth.val - 1 ∧
    ∃ (n : Usize) (c : U8), n.val = s.lock_len.val - 1 ∧ IsCeil cfg s n.val c.val ∧
      s' = { s with ceiling := c, lock_len := n } := by
  simp only [logic.sched.Sched.unlock] at h
  have h1 : (1#usize).val = 1 := rfl
  split at h
  · cases (Prod.mk.inj (ok_inj h)).1
  · rename_i hlen
    simp only [bind_eq_ok] at h
    obtain ⟨n, hn, i, hi, h⟩ := h
    obtain ⟨hn1, hn2⟩ := usize_sub_ok hn
    rw [h1] at hn1 hn2
    obtain ⟨_, rfl⟩ := index_ok hi
    split at h
    · cases (Prod.mk.inj (ok_inj h)).1
    · rename_i hres
      simp only [bind_eq_ok] at h
      obtain ⟨i1, hi1, i2, hi2, h⟩ := h
      obtain ⟨_, rfl⟩ := index_ok hi1
      obtain ⟨hd1, hd2⟩ := usize_sub_ok hi2
      rw [h1] at hd1 hd2
      split at h
      · cases (Prod.mk.inj (ok_inj h)).1
      · rename_i hdep
        simp only [bind_eq_ok] at h
        obtain ⟨c, hc, h⟩ := h
        have h := (Prod.mk.inj (ok_inj h)).2
        have hceil := recompute_ok hc
        refine ⟨hn1, hd1, ?_, ?_, n, c, hn2, hceil, h.symm⟩
        · simp only [lres]; rw [← hn2]
          simpa using hres
        · simp only [ldep]; rw [← hn2, ← hd2]
          simpa using hdep

/-- Job end: the Job's lock entries are popped, the System_Ceiling
recomputed, and the Job leaves the stack. -/
theorem finish_top_ok {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {rel : Usize} (h : logic.sched.Sched.finish_top s cfg = ok (rel, s')) :
    1 ≤ s.depth.val ∧ s.depth.val ≤ NT.val ∧ (s.stack.val[s.depth.val - 1]!).val < NT.val ∧
    ∃ (d n : Usize) (c : U8), d.val = s.depth.val - 1 ∧ n.val ≤ s.lock_len.val ∧
      (∀ p, n.val ≤ p → p < s.lock_len.val → ldep s p = s.depth.val - 1) ∧
      (n.val = 0 ∨ ldep s (n.val - 1) ≠ s.depth.val - 1) ∧ IsCeil cfg s n.val c.val ∧
      s' = { s with
        job := s.job.set (UScalar.cast .Usize (s.stack.val[s.depth.val - 1]!)) logic.sched.JOB_NONE,
        depth := d, ceiling := c, lock_len := n } := by
  simp only [logic.sched.Sched.finish_top, logic.sched.Sched.end_job, bind_eq_ok] at h
  obtain ⟨d0, hd0, n, hn, released, hrel, c, hc, s1, hs1, h⟩ := h
  obtain ⟨_, hd0v⟩ := usize_sub_ok hd0
  have h1 : (1#usize).val = 1 := rfl
  rw [h1] at hd0v
  obtain ⟨g1, g2, g3⟩ := end_job_loop_ok s.lock_depth d0 _ s.lock_len n rfl hn
  have hceil := recompute_ok hc
  obtain ⟨c1, c2, c3, d, hd, rfl⟩ := complete_ok hs1
  have h := (Prod.mk.inj (ok_inj h)).2
  refine ⟨c1, c2, c3, d, n, c, hd, g1, ?_, ?_, hceil, h.symm⟩
  · intro p hp1 hp2
    simp only [ldep]
    rw [g2 p hp1 hp2, hd0v]
  · rcases g3 with g | g
    · left; exact g
    · right
      intro hv
      apply g
      simp only [ldep] at hv
      scalar_tac

end P18
