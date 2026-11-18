import Sched.Settle
open Aeneas Aeneas.Std Result

/-! # Mutual exclusion and the ceiling protocol on the Kernel's code (2026-11-18)

Properties 3 and 4 of the design, which Verus proves of `rsk_logic`
(`Sched::inv`, `lock`, `unlock`), stated over the run of the Kernel's
functions as Aeneas extracts them:

- Property 4: in every state a run reaches no Resource is held twice
  (`kreach_excl`: so by no two Jobs, each entry of the lock stack having
  one holder), and a lock entry finds its Resource free of every other Job
  (`lock_entry_finds_free`: an entry that holds the Resource can only be
  the caller's own, which `Sched::lock` refuses, `lock_free`).
- Property 3: the System_Ceiling is the maximum of the Ceilings of the
  held Resources (`kreach_ceiling`), a granted lock raises it to the
  maximum of its value and the Resource's Ceiling (`lock_raises`), and the
  unlock that matches a lock returns it to the value before that lock
  (`unlock_restores`). -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-! ## What a granted lock found -/

/-- `Sched::lock`'s search of the lock stack: when it finds no entry, none
of the entries it looked at holds the Resource. -/
theorem lock_loop_ok {a : Array U8 NL} {i : Usize} {r : U8} :
    ∀ (n : Nat) (e : Usize) (re res : Bool), i.val - e.val = n →
      logic.sched.Sched.lock_loop a i r e re = ok res →
      res = false → re = false ∧ ∀ p, e.val ≤ p → p < i.val → a.val[p]! ≠ r := by
  intro n
  induction n with
  | zero =>
    intro e re res hn h hf
    unfold logic.sched.Sched.lock_loop loop at h
    simp only [logic.sched.Sched.lock_loop.body] at h
    have hlt : ¬ e < i := by scalar_tac
    simp only [hlt, ite_false, bind_ok] at h
    have h := ok_inj h
    subst h
    exact ⟨hf, fun p h1 h2 => by omega⟩
  | succ n ih =>
    intro e re res hn h hf
    unfold logic.sched.Sched.lock_loop loop at h
    simp only [logic.sched.Sched.lock_loop.body] at h
    have hlt : e < i := by scalar_tac
    simp only [hlt, ite_true, lift, bind_ok, bind_eq_ok] at h
    obtain ⟨cf, ⟨x, hx, re1, hre1, e1, he1, hcf⟩, hloop⟩ := h
    have hcf := ok_inj hcf
    subst hcf
    have he1v := usize_add_ok he1
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at he1v
    have hrec : logic.sched.Sched.lock_loop a i r e1 re1 = ok res := by
      unfold logic.sched.Sched.lock_loop
      simp only [logic.sched.Sched.lock_loop.body]
      exact hloop
    obtain ⟨hre1f, hall⟩ := ih e1 re1 res (by omega) hrec hf
    obtain ⟨_, hxv⟩ := index_ok hx
    split at hre1
    · have := ok_inj hre1
      subst this
      cases hre1f
    · rename_i hne
      have := ok_inj hre1
      subst this
      refine ⟨hre1f, fun p hp1 hp2 => ?_⟩
      by_cases hp : p = e.val
      · subst hp
        rw [← hxv]
        exact hne
      · exact hall p (by omega) hp2

/-- The Task `t` is a declared accessor of the Resource `r`. -/
def acc (cfg : logic.Config NT NR NP) (t r : Nat) : Prop :=
  ((cfg.access.val[t]!).val[r]!) = true

/-- A granted lock found the Resource in no entry of the lock stack, and
its caller, the Job on top of the stack, is a declared accessor. -/
theorem lock_free {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.lock s cfg r = ok (true, s')) :
    (∀ p, p < s.lock_len.val → lres s p ≠ r.val) ∧ acc cfg (stk s (s.depth.val - 1)) r.val := by
  simp only [logic.sched.Sched.lock, lift, bind_ok, bind_eq_ok] at h
  obtain ⟨d, hd, t, ht, a, ha, b, hb, h⟩ := h
  obtain ⟨_, hd2⟩ := usize_sub_ok hd
  have h1 : (1#usize).val = 1 := rfl
  rw [h1] at hd2
  obtain ⟨_, ht'⟩ := index_ok ht
  obtain ⟨_, ha'⟩ := index_ok ha
  obtain ⟨_, hb'⟩ := index_ok hb
  split at h
  · rename_i hacc
    simp only [bind_eq_ok] at h
    obtain ⟨re, hre, h⟩ := h
    split at h
    · cases (Prod.mk.inj (ok_inj h)).1
    · rename_i hre0
      have hf : re = false := by simpa using hre0
      obtain ⟨_, hall⟩ := lock_loop_ok _ _ _ _ rfl hre hf
      refine ⟨fun p hp hc => ?_, ?_⟩
      · simp only [lres] at hc
        exact hall p (by scalar_tac) hp (by scalar_tac)
      · unfold acc stk
        rw [← hd2, ← ht']
        rw [hb', ha'] at hacc
        simpa [U8.cast_Usize_val_eq] using hacc
  · cases (Prod.mk.inj (ok_inj h)).1

/-! ## Property 4: mutual exclusion without waiting -/

/-- No Resource is in two entries of the lock stack: none is held twice,
by one Job or by two. -/
def Excl (s : logic.sched.Sched NT NR NL) : Prop :=
  ∀ p q, p < q → q < s.lock_len.val → lres s p ≠ lres s q

theorem excl_same {s s' : logic.sched.Sched NT NR NL} (h : LockSame s' s) (he : Excl s) :
    Excl s' := by
  obtain ⟨hl, _, hn, _⟩ := h
  intro p q hpq hq
  rw [hl p, hl q]
  exact he p q hpq (by rw [← hn]; exact hq)

theorem excl_lock {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.lock s cfg r = ok (true, s')) (he : Excl s) : Excl s' := by
  obtain ⟨_, _, _, hn, hl, _, _⟩ := lock_views h
  obtain ⟨hfree, _⟩ := lock_free h
  intro p q hpq hq
  rw [hl p, hl q]
  have hp : ¬ p = s.lock_len.val := by omega
  simp only [hp, ite_false]
  by_cases hq' : q = s.lock_len.val
  · simp only [hq', ite_true]
    exact hfree p (by omega)
  · simp only [hq', ite_false]
    exact he p q hpq (by omega)

theorem excl_unlock {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.unlock s cfg r = ok (true, s')) (he : Excl s) : Excl s' := by
  obtain ⟨_, _, _, _, _, hn, hl, _, _⟩ := unlock_views h
  intro p q hpq hq
  rw [hl p, hl q]
  exact he p q hpq (by omega)

theorem excl_finish_top {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r : Usize} (h : logic.sched.Sched.finish_top s cfg = ok (r, s')) (he : Excl s) : Excl s' := by
  obtain ⟨_, _, _, _, _, _, _, hl, _, hn, _, _, _⟩ := finish_top_views h
  intro p q hpq hq
  rw [hl p, hl q]
  exact he p q hpq (by omega)

/-- Every transition of the run keeps it. -/
theorem excl_step {cfg : logic.Config NT NR NP} {P : Params} {st st' : St NT NR NL}
    (he : Excl st.s) (h : Step cfg P st st') : Excl st'.s := by
  cases h <;> dsimp only <;> first
    | exact he
    | exact excl_same (make_pending_views (by assumption)).2.2.2.2.2 he
    | exact excl_same (defer_views (by assumption)).2.2.2.2.2 he
    | exact excl_same (start_views (by assumption)).2.2.2.2.2.2 he
    | exact excl_same (kill_views (by assumption)).2.2.2.2.2 he
    | exact excl_same (clear_release_views (by assumption)).2.2.2.2.2 he
    | exact excl_lock (by assumption) he
    | exact excl_unlock (by assumption) he
    | exact excl_finish_top (by assumption) he

theorem reach_excl {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (h : Reach cfg P st) : Excl st.s := by
  induction h with
  | boot s0 h0 =>
    obtain ⟨_, _, _, _, hlock⟩ := new_views h0
    intro p q _ hq
    dsimp only [St.boot] at hq
    omega
  | step st st' _ hs ih => exact excl_step ih hs

/-- Property 4, first half: in every state a run of the Kernel's functions
reaches, no Resource is held twice. -/
theorem kreach_excl {cfg : logic.Config NT NR NP} {P : Params} (hNT : NT.val ≤ 256)
    {x : KSt NT NR NP NL NS NE} (h : KReach cfg P x) : Excl x.k.sched :=
  reach_excl (ksim hNT h)

/-- Property 4, second half: a lock entry finds its Resource free. In a
state a run reaches, an entry of the lock stack that holds a Resource the
Job on top may access is that Job's own: no other Job holds it. Under the
Generator's rule that a Resource's Ceiling is at least the Priority of
each of its accessors (R23.1). `Sched::lock` then refuses the Job's own
(`lock_free`: a granted lock found the Resource in no entry at all). -/
theorem lock_entry_finds_free {cfg : logic.Config NT NR NP} {P : Params} {st : St NT NR NL}
    (w : Wf cfg P st) (hceil : ∀ t r, acc cfg t r → prio cfg t ≤ ceil cfg r) {r : Nat}
    (hd : 0 < st.s.depth.val) (ha : acc cfg (stk st.s (st.s.depth.val - 1)) r) :
    ∀ p, p < st.s.lock_len.val → lres st.s p = r → ldep st.s p = st.s.depth.val - 1 := by
  intro p hp hr
  by_contra hne
  have hlt := w.ldep_lt p hp
  have hsrp := w.srp p (st.s.depth.val - 1) hp (by omega) (by omega)
  rw [hr] at hsrp
  have := hceil _ _ ha
  omega

theorem kreach_lock_entry_finds_free {cfg : logic.Config NT NR NP} {P : Params}
    (hNT : NT.val ≤ 256) {x : KSt NT NR NP NL NS NE} (h : KReach cfg P x)
    (hceil : ∀ t r, acc cfg t r → prio cfg t ≤ ceil cfg r) {r : Nat}
    (hd : 0 < x.k.sched.depth.val) (ha : acc cfg (stk x.k.sched (x.k.sched.depth.val - 1)) r) :
    ∀ p, p < x.k.sched.lock_len.val → lres x.k.sched p = r →
      ldep x.k.sched p = x.k.sched.depth.val - 1 :=
  lock_entry_finds_free (reach_wf (ksim hNT h)) hceil hd ha

/-! ## Property 3: ceiling raise, restore, and LIFO nesting -/

/-- The System_Ceiling of a lock stack is determined by it. -/
theorem IsCeil.unique {cfg : logic.Config NT NR NP} {s : logic.sched.Sched NT NR NL} {n c c' : Nat}
    (h : IsCeil cfg s n c) (h' : IsCeil cfg s n c') : c = c' := by
  obtain ⟨h1, h2⟩ := h
  obtain ⟨h1', h2'⟩ := h'
  apply Nat.le_antisymm
  · rcases h2 with h0 | ⟨p, hp, hc⟩
    · omega
    · rw [hc]; exact h1' p hp
  · rcases h2' with h0 | ⟨p, hp, hc⟩
    · omega
    · rw [hc]; exact h1 p hp

/-- and only by its first `n` entries. -/
theorem IsCeil.congr {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL} {n c : Nat}
    (hl : ∀ p, p < n → lres s' p = lres s p) (h : IsCeil cfg s n c) : IsCeil cfg s' n c := by
  obtain ⟨h1, h2⟩ := h
  refine ⟨fun p hp => by rw [hl p hp]; exact h1 p hp, ?_⟩
  rcases h2 with h0 | ⟨p, hp, hc⟩
  · exact Or.inl h0
  · exact Or.inr ⟨p, hp, by rw [hl p hp]; exact hc⟩

/-- In every state a run reaches, the System_Ceiling is the greatest
Ceiling of the held Resources, 0 when none is held. -/
theorem kreach_ceiling {cfg : logic.Config NT NR NP} {P : Params} (hNT : NT.val ≤ 256)
    {x : KSt NT NR NP NL NS NE} (h : KReach cfg P x) :
    IsCeil cfg x.k.sched x.k.sched.lock_len.val x.k.sched.ceiling.val :=
  (reach_wf (ksim hNT h)).ceil_is

/-- R8.1: a granted lock raises the System_Ceiling to the maximum of its
value and the Resource's Ceiling. -/
theorem lock_raises {s s' : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP} {r : U8}
    (h : logic.sched.Sched.lock s cfg r = ok (true, s')) :
    s'.ceiling.val = max s.ceiling.val (ceil cfg r.val) :=
  (lock_views h).2.2.2.2.2.2

/-- R8.2, R8.6: the unlock that matches a lock returns the System_Ceiling
to its value before that lock. `s` is a state whose System_Ceiling is its
lock stack's (every reached state, `kreach_ceiling`), the lock is granted
in it, and `s2` is any later state whose lock stack is again the one the
lock left (the sections entered in between have been left, in LIFO
order); the unlock granted there leaves the System_Ceiling of `s`. -/
theorem unlock_restores {s s1 s2 s3 : logic.sched.Sched NT NR NL} {cfg : logic.Config NT NR NP}
    {r r' : U8} (hc : IsCeil cfg s s.lock_len.val s.ceiling.val)
    (hlock : logic.sched.Sched.lock s cfg r = ok (true, s1))
    (hlen : s2.lock_len.val = s1.lock_len.val)
    (hres : ∀ p, p < s1.lock_len.val → lres s2 p = lres s1 p)
    (hunlock : logic.sched.Sched.unlock s2 cfg r' = ok (true, s3)) :
    s3.ceiling.val = s.ceiling.val ∧ r'.val = r.val := by
  obtain ⟨_, _, _, hn1, hl1, _, _⟩ := lock_views hlock
  obtain ⟨_, _, _, htop, _, _, _, _, hc3⟩ := unlock_views hunlock
  have hlen2 : s2.lock_len.val - 1 = s.lock_len.val := by omega
  rw [hlen2] at hc3 htop
  have hsame : ∀ p, p < s.lock_len.val → lres s p = lres s2 p := by
    intro p hp
    rw [hres p (by omega), hl1 p]
    have : ¬ p = s.lock_len.val := by omega
    simp only [this, ite_false]
  refine ⟨IsCeil.unique (IsCeil.congr hsame hc3) hc, ?_⟩
  rw [← htop, hres _ (by omega), hl1]
  simp

end P18
