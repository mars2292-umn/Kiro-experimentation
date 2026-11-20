import Sched.ArchIrq
import Sched.Timer
import Sched.Settle
open Aeneas Aeneas.Std Result

/-! # The Kernel behind the trait satisfies the trait's contract (2026-11-20)

The entry cores are proven for any Kernel that satisfies `KSpec`/`KSpecD`
(`Sched/ArchCores.lean`, `Sched/ArchSvcKeep.lean`). This file discharges
those clauses for the Kernel that is linked: the methods of
`impl KernelOps for Bound` as Aeneas extracts them.

- `kvB` reads a `Bound` as its preemption stack and configuration; `okB` is
  the part of the Kernel's invariant Kernel_Arch needs (the stack holds
  Task identifiers in strictly rising Priority; the configuration's
  Priorities are levels 1 to 7 and its Partitions identifiers).
- Each method leaves the stack a prefix of what it was (`SP`), or exactly
  what it was (`SS`), except `dispatch`, which pushes an eligible Task: one
  of higher Priority than the Job on top.
- `bound_kspec`: a trait instance whose methods are the extracted ones
  satisfies `KSpecD`.

Since 2026-11-20 the extraction holds `top`, `boot_failed` and `boot_info`
too, so `IsBound` is only equations between `ki`'s methods and the
extracted ones. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The stack of `s'` is that of `s` with Jobs ended from the top. -/
def SP (s' s : logic.sched.Sched NT NR NL) : Prop :=
  s'.depth.val ≤ s.depth.val ∧ ∀ p, stk s' p = stk s p

/-- The stack of `s'` is that of `s`. -/
def SS (s' s : logic.sched.Sched NT NR NL) : Prop :=
  s'.depth.val = s.depth.val ∧ ∀ p, stk s' p = stk s p

theorem SP.refl (s : logic.sched.Sched NT NR NL) : SP s s := ⟨Nat.le_refl _, fun _ => rfl⟩

theorem SP.trans {a b c : logic.sched.Sched NT NR NL} (h1 : SP b a) (h2 : SP c b) : SP c a :=
  ⟨Nat.le_trans h2.1 h1.1, fun p => (h2.2 p).trans (h1.2 p)⟩

theorem SS.refl (s : logic.sched.Sched NT NR NL) : SS s s := ⟨rfl, fun _ => rfl⟩

theorem SS.trans {a b c : logic.sched.Sched NT NR NL} (h1 : SS b a) (h2 : SS c b) : SS c a :=
  ⟨h2.1.trans h1.1, fun p => (h2.2 p).trans (h1.2 p)⟩

theorem SS.sp {a b : logic.sched.Sched NT NR NL} (h : SS b a) : SP b a :=
  ⟨Nat.le_of_eq h.1, h.2⟩

theorem SS.of_eq {a b : logic.sched.Sched NT NR NL} (h : b = a) : SS b a := by
  subst h; exact SS.refl _

/-! ## The scheduler's steps -/

theorem rstep_sp {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL}
    (h : RStep cfg s s') : SP s' s := by
  cases h with
  | kend r h =>
    have hv := finish_top_views h
    exact ⟨by rw [hv.2.2.2.2.2.2.1]; omega, hv.2.2.2.2.2.1⟩
  | kkill p hp h =>
    obtain ⟨_, _, rfl⟩ := kill_ok h
    exact ⟨Nat.le_refl _, fun _ => rfl⟩
  | kclear t h =>
    obtain ⟨_, rfl⟩ := clear_release_ok h
    exact ⟨Nat.le_refl _, fun _ => rfl⟩

theorem rsteps_sp {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL}
    (h : RSteps cfg s s') : SP s' s := by
  induction h with
  | refl => exact SP.refl _
  | tail _ hstep ih => exact ih.trans (rstep_sp hstep)

theorem make_pending_ss {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL} {t : U8}
    (h : logic.sched.Sched.make_pending s cfg t = ok s') : SS s' s := by
  obtain ⟨_, rfl⟩ := make_pending_ok h
  exact ⟨rfl, fun _ => rfl⟩

theorem defer_ss {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL} {t : U8}
    (h : logic.sched.Sched.defer s cfg t = ok s') : SS s' s := by
  obtain ⟨_, rfl⟩ := defer_ok h
  exact ⟨rfl, fun _ => rfl⟩

theorem clear_release_ss {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL}
    {t : U8} (h : logic.sched.Sched.clear_release s cfg t = ok s') : SS s' s := by
  obtain ⟨_, rfl⟩ := clear_release_ok h
  exact ⟨rfl, fun _ => rfl⟩

set_option maxHeartbeats 2000000 in
/-- `Sched::lock`, granted or refused, leaves the stack. -/
theorem lock_ss {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL} {r : U8}
    {b : Bool} (h : logic.sched.Sched.lock s cfg r = ok (b, s')) : SS s' s := by
  unfold logic.sched.Sched.lock at h
  peel h
  all_goals exact ⟨rfl, fun _ => rfl⟩

set_option maxHeartbeats 2000000 in
/-- `Sched::unlock`, granted or refused, leaves the stack. -/
theorem unlock_ss {cfg : logic.Config NT NR NP} {s s' : logic.sched.Sched NT NR NL} {r : U8}
    {b : Bool} (h : logic.sched.Sched.unlock s cfg r = ok (b, s')) : SS s' s := by
  unfold logic.sched.Sched.unlock at h
  peel h
  all_goals exact ⟨rfl, fun _ => rfl⟩

/-! ## The Kernel's entries -/

variable {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP}

theorem safe_loop_ss {s s' : logic.sched.Sched NT NR NL} {t : Usize}
    (h : logic.kernel.Kernel.enter_safe_state_loop s cfg t = ok s') : SS s' s := by
  unfold logic.kernel.Kernel.enter_safe_state_loop at h
  refine loop_inv _ (fun x => SS x.1 s) (fun r => SS r s) ?_ ?_ (s, t) s' (SS.refl s) h
  · rintro ⟨st, c⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.enter_safe_state_loop.body at hb
    peel hb
    exact hinv.trans (clear_release_ss (by assumption))
  · rintro ⟨st, c⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.enter_safe_state_loop.body at hb
    peel hb
    exact hinv

/-- The safe state ends no Job: it drops Releases. -/
theorem enter_safe_state_ss (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') :
    SS k'.sched k.sched := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  peel h
  · dsimp only
    exact safe_loop_ss (by assumption)
  · dsimp only
    have := safe_loop_ss (cfg := cfg) (by assumption)
    rw [push_event_sched (by assumption)] at this
    exact this

/-- A fault attributed to the Kernel ends no Job. -/
theorem fault_kernel_ss {kind : U8}
    (h : logic.kernel.Kernel.fault k cfg kind true = ok k') : SS k'.sched k.sched := by
  unfold logic.kernel.Kernel.fault at h
  simp only [↓reduceIte] at h
  peel h
  have hr := enter_safe_state_ss h
  have hp := push_event_sched ‹k.push_event cfg kind 255#u8 255#u8 _ 0#u32 = ok _›
  rw [hp] at hr
  exact hr

theorem svc_lock_ss {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_lock k cfg r = ok (b, k')) : SS k'.sched k.sched :=
  lock_ss (svc_lock_eff h)

theorem svc_unlock_ss {r : U8} {b : Bool}
    (h : logic.kernel.Kernel.svc_unlock k cfg r = ok (b, k')) : SS k'.sched k.sched :=
  unlock_ss (svc_unlock_eff h)

theorem kjob_return_sp {ec : U32}
    (h : logic.kernel.Kernel.job_return k cfg ec = ok k') :
    SP k'.sched k.sched ∧ k'.sched.depth.val = k.sched.depth.val - 1 := by
  obtain ⟨r, hr⟩ := job_return_eff h
  have hv := finish_top_views hr
  exact ⟨⟨by rw [hv.2.2.2.2.2.2.1]; omega, hv.2.2.2.2.2.1⟩, hv.2.2.2.2.2.2.1⟩

theorem intake_release_sp' {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake_release k cfg t sp now off = ok k') :
    SP k'.sched k.sched := by
  rcases intake_release_eff h with ⟨_, hm⟩ | ⟨s1, _, hd, hr⟩ | hr
  · exact (make_pending_ss hm).sp
  · exact (defer_ss hd).sp.trans (rsteps_sp hr)
  · exact rsteps_sp hr

set_option maxHeartbeats 2000000 in
theorem intake_sp {t sp off : U8} {now : U64}
    (h : logic.kernel.Kernel.intake k cfg t sp now off = ok k') : SP k'.sched k.sched := by
  unfold logic.kernel.Kernel.intake at h
  peel h
  all_goals
    have ho := observe_sched (by assumption)
    have hi := intake_release_sp' h
    first
      | (rw [ho] at hi; exact hi)
      | (have hd := rsteps_sp (deadline_miss_eff (by assumption))
         rw [ho] at hd
         exact hd.trans hi)

theorem serve_slot_sp {i : Usize}
    (h : logic.kernel.Kernel.serve_slot k cfg i = ok k') : SP k'.sched k.sched := by
  cases serve_slot_cases h with
  | release _ h =>
    rcases periodic_release_eff h with ⟨_, hm⟩ | hr
    · exact (make_pending_ss hm).sp
    · exact rsteps_sp hr
  | defer _ h =>
    rcases deferred_release_eff h with ⟨_, hm⟩ | hr
    · exact (make_pending_ss hm).sp
    · exact rsteps_sp hr
  | deadline _ h => exact rsteps_sp (deadline_miss_eff h)
  | budget _ h => exact rsteps_sp (budget_event_eff h)
  | opend _ h => exact rsteps_sp h
  | kept _ _ h => exact (SS.of_eq h).sp

theorem served_chain_sp (h : ServedChain cfg k k') : SP k'.sched k.sched := by
  induction h with
  | done => exact SP.refl _
  | serve k k1 k' i _ hs _ ih => exact (serve_slot_sp hs).trans ih

theorem timer_event_sp {now : U64} {rc : U32}
    (h : logic.kernel.Kernel.timer_event k cfg now rc = ok k') : SP k'.sched k.sched := by
  obtain ⟨k1, hs, hc⟩ := timer_event_served h
  have := served_chain_sp hc
  rw [hs] at this
  exact this

theorem start_loop_sched {t : Usize} {cfg' : logic.Config NT NR NP}
    (h : logic.kernel.Kernel.start_loop k cfg t = ok (k', cfg')) : k'.sched = k.sched := by
  unfold logic.kernel.Kernel.start_loop at h
  refine loop_inv _ (fun x => x.1.sched = k.sched) (fun r => r.1.sched = k.sched) ?_ ?_ (k, t)
    (k', cfg') rfl h
  · rintro ⟨st, c⟩ ⟨st', c'⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.start_loop.body at hb
    peel hb
    all_goals exact hinv
  · rintro ⟨st, c⟩ ⟨y, z⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.start_loop.body at hb
    peel hb
    exact hinv

set_option maxHeartbeats 2000000 in
/-- `Kernel::start` arms slots: the scheduler state is as it was. -/
theorem kernel_start_sched (h : logic.kernel.Kernel.start k cfg = ok k') :
    k'.sched = k.sched := by
  unfold logic.kernel.Kernel.start at h
  peel h
  all_goals (dsimp only; exact start_loop_sched (by assumption))

/-! ## The methods of `impl KernelOps for Bound` -/

/-- The Partition of Task `k` in the Kernel's configuration (0 out of range). -/
def partOf (cfg : logic.Config NT NR NP) (k : Nat) : Nat :=
  match cfg.tasks.val[k]? with
  | some tc => tc.partition.val
  | none => 0

/-- A `Bound` read as its preemption stack and configuration. -/
def kvB (b : arch.bound.Bound NT NR NP NL NS NE) : Arch.KV where
  depth := b.kernel.sched.depth.val
  stack := stk b.kernel.sched
  prio := prio b.cfg
  part := partOf b.cfg

/-- The part of the Kernel's invariant Kernel_Arch needs. -/
def okB (b : arch.bound.Bound NT NR NP NL NS NE) : Prop :=
  NT.val ≤ 255 ∧
  (∀ t, t < NT.val → 1 ≤ (kvB b).prio t ∧ (kvB b).prio t < 8 ∧ (kvB b).part t < 255) ∧
  (∀ i, i < (kvB b).depth → (kvB b).stack i < NT.val) ∧
  (∀ i j, i < j → j < (kvB b).depth →
    (kvB b).prio ((kvB b).stack i) < (kvB b).prio ((kvB b).stack j))

theorem okB_kok {b : arch.bound.Bound NT NR NP NL NS NE} (h : okB b) : Arch.KOk (kvB b) := by
  obtain ⟨h1, h2, h3, h4⟩ := h
  refine ⟨h4, fun i hi => ?_⟩
  have hs := h3 i hi
  obtain ⟨a, b', c⟩ := h2 _ hs
  exact ⟨by omega, a, b', c⟩

/-- A method that keeps the configuration and leaves the stack a prefix
keeps the invariant. -/
theorem okB_sp {b b' : arch.bound.Bound NT NR NP NL NS NE} (hc : b'.cfg = b.cfg)
    (hsp : SP b'.kernel.sched b.kernel.sched) : KR kvB okB b' b := by
  intro hok
  have hpr : (kvB b').prio = (kvB b).prio := by simp only [kvB, hc]
  have hpa : (kvB b').part = (kvB b).part := by simp only [kvB, hc]
  have hst : ∀ p, (kvB b').stack p = (kvB b).stack p := hsp.2
  have hd : (kvB b').depth ≤ (kvB b).depth := hsp.1
  obtain ⟨h1, h2, h3, h4⟩ := hok
  refine ⟨⟨h1, fun t ht => ?_, fun i hi => ?_, fun i j hij hj => ?_⟩, hd, fun i _ => hst i, hpr,
    hpa⟩
  · rw [hpr, hpa]; exact h2 t ht
  · rw [hst]; exact h3 i (by omega)
  · rw [hpr, hst, hst]; exact h4 i j hij (by omega)

/-- What a method that is not a dispatch leaves: the same configuration and
a prefix of the stack. -/
def BP (b' b : arch.bound.Bound NT NR NP NL NS NE) : Prop :=
  b'.cfg = b.cfg ∧ SP b'.kernel.sched b.kernel.sched

/-- The same configuration and the same stack. -/
def BS (b' b : arch.bound.Bound NT NR NP NL NS NE) : Prop :=
  b'.cfg = b.cfg ∧ SS b'.kernel.sched b.kernel.sched

theorem BS.bp {b b' : arch.bound.Bound NT NR NP NL NS NE} (h : BS b' b) : BP b' b :=
  ⟨h.1, h.2.sp⟩

theorem BP.kr {b b' : arch.bound.Bound NT NR NP NL NS NE} (h : BP b' b) : KR kvB okB b' b :=
  okB_sp h.1 h.2

theorem BS.depth {b b' : arch.bound.Bound NT NR NP NL NS NE} (h : BS b' b) :
    (kvB b').depth = (kvB b).depth := h.2.1

variable {b b' : arch.bound.Bound NT NR NP NL NS NE}

theorem bound_start_bs
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start b = ok b') : BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start at h
  peel h
  · exact ⟨rfl, SS.refl _⟩
  · exact ⟨rfl, SS.of_eq (kernel_start_sched (by assumption))⟩

theorem bound_job_return_bp {ec : U32}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return b ec = ok b') :
    BP b' b ∧ (0 < (kvB b).depth → (kvB b').depth = (kvB b).depth - 1) := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return at h
  peel h
  · obtain ⟨hsp, hd⟩ := kjob_return_sp (by assumption)
    exact ⟨⟨rfl, hsp⟩, fun _ => hd⟩
  · rename_i hn
    refine ⟨⟨rfl, SP.refl _⟩, fun hp => ?_⟩
    exfalso
    apply hn
    simp only [kvB] at hp
    scalar_tac

theorem bound_svc_lock_bs {r : U8} {x : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock b r = ok (x, b')) :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock at h
  peel h
  all_goals first
    | exact ⟨rfl, SS.refl _⟩
    | exact ⟨rfl, svc_lock_ss (by assumption)⟩

theorem bound_svc_unlock_bs {r : U8} {x : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock b r = ok (x, b')) :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock at h
  peel h
  all_goals first
    | exact ⟨rfl, SS.refl _⟩
    | exact ⟨rfl, svc_unlock_ss (by assumption)⟩

theorem bound_settle_bs {x : Option U8 × Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle b = ok (x, b')) :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle at h
  peel h
  all_goals exact ⟨rfl, SS.of_eq (settle_sched (by assumption))⟩

set_option maxHeartbeats 2000000 in
theorem bound_intake_bp {t sp off : U8} {now : U64}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake b t sp now off = ok b') :
    BP b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake at h
  peel h
  all_goals first
    | exact ⟨rfl, SP.refl _⟩
    | exact ⟨rfl, intake_sp (by assumption)⟩

theorem bound_timer_event_bp {now : U64} {rc : U32}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event b now rc = ok b') :
    BP b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event at h
  peel h
  all_goals exact ⟨rfl, timer_event_sp (by assumption)⟩

theorem bound_fault_bp {kind : U8} {kn : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind kn = ok b') :
    BP b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault at h
  peel h
  exact ⟨rfl, rsteps_sp (fault_r (by assumption))⟩

theorem bound_fault_kernel_bs {kind : U8}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault b kind true = ok b') :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault at h
  peel h
  exact ⟨rfl, fault_kernel_ss (by assumption)⟩

theorem bound_take_reinit_bs {p : U8} {x : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit b p = ok (x, b')) :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit at h
  peel h
  all_goals first
    | exact ⟨rfl, SS.refl _⟩
    | exact ⟨rfl, SS.of_eq (ack_reinit_frame (by assumption)).1⟩

theorem bound_take_stop_dma_bs {p : U8} {x : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma b p = ok (x, b')) :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma at h
  peel h
  all_goals first
    | exact ⟨rfl, SS.refl _⟩
    | exact ⟨rfl, SS.of_eq (ack_stop_dma_frame (by assumption)).1⟩

theorem bound_record_event_bs {kind t : U8}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event b kind t = ok b') :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event at h
  peel h
  all_goals first
    | exact ⟨rfl, SS.refl _⟩
    | exact ⟨rfl, SS.of_eq (record_event_sched (by assumption))⟩

theorem bound_fault_of_bp {kind t : U8}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of b kind t = ok b') :
    BP b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of at h
  peel h
  all_goals first
    | exact ⟨rfl, SP.refl _⟩
    | exact ⟨rfl, rsteps_sp (fault_of_r (by assumption))⟩

theorem bound_depth {d : Usize}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.depth b = ok d) :
    d.val = (kvB b).depth := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.depth at h
  have h := ok_inj h
  rw [← h]; rfl

theorem bound_job_info {t : U8} {ji : arch.bound.JobInfo}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_info b t = ok ji) :
    ji.priority.val = (kvB b).prio t.val ∧ ji.partition.val = (kvB b).part t.val := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_info at h
  peel h
  · rename_i hlt task htask
    have hv := (index_ok' htask).2
    rw [U8.cast_Usize_val_eq] at hv
    simp [kvB, prio, partOf, hv]
  · rename_i hge
    have hlen : b.cfg.tasks.val.length = NT.val := b.cfg.tasks.property
    have hv : b.cfg.tasks.val[t.val]? = none := List.getElem?_eq_none (by scalar_tac)
    simp [kvB, prio, partOf, hv]

theorem bound_top {o : Option U8} (hok : okB b)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.top b = ok o) :
    (o = none ↔ (kvB b).depth = 0) ∧ ∀ t, o = some t → t.val = (kvB b).stack ((kvB b).depth - 1) := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.top at h
  split at h
  · rename_i hz
    have h := ok_inj h
    subst h
    refine ⟨⟨fun _ => by simp only [kvB]; scalar_tac, fun _ => rfl⟩, fun t ht => by cases ht⟩
  · rename_i hnz
    simp only [bind_eq_ok] at h
    obtain ⟨i, hi, i1, hi1, h⟩ := h
    have h := ok_inj h
    subst h
    obtain ⟨_, hiv⟩ := usize_sub_ok hi
    have h1 : (1#usize).val = 1 := rfl
    rw [h1] at hiv
    obtain ⟨_, rfl⟩ := index_ok hi1
    refine ⟨⟨fun e => (by cases e), fun e => ?_⟩, fun t ht => ?_⟩
    · exfalso
      apply hnz
      simp only [kvB] at e
      scalar_tac
    · cases ht
      simp only [kvB, stk]
      rw [hiv]

theorem boot_check_failed_ss {failed : U32}
    (h : logic.kernel.Kernel.boot_check_failed k cfg failed = ok k') : SS k'.sched k.sched := by
  unfold logic.kernel.Kernel.boot_check_failed at h
  simp only [bind_eq_ok] at h
  obtain ⟨k1, hk1, h⟩ := h
  have hr := enter_safe_state_ss h
  rw [push_event_sched hk1] at hr
  exact hr

theorem boot_info_sched {x y : Bool}
    (h : logic.kernel.Kernel.boot_info k cfg x y = ok k') : k'.sched = k.sched := by
  unfold logic.kernel.Kernel.boot_info at h
  simp only [bind_eq_ok] at h
  obtain ⟨k1, hk1, h⟩ := h
  have e1 : k1.sched = k.sched := by
    split at hk1
    · exact push_event_sched hk1
    · have := ok_inj hk1; rw [← this]
  split at h
  · exact (push_event_sched h).trans e1
  · have := ok_inj h; rw [← this]; exact e1

theorem bound_boot_failed_bs {f : U32}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed b f = ok b') :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed at h
  simp only [bind_eq_ok] at h
  obtain ⟨k1, hk1, h⟩ := h
  have h := ok_inj h
  subst h
  exact ⟨rfl, boot_check_failed_ss hk1⟩

theorem bound_boot_info_bs {x y : Bool}
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info b x y = ok b') :
    BS b' b := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info at h
  simp only [bind_eq_ok] at h
  obtain ⟨k1, hk1, h⟩ := h
  have h := ok_inj h
  subst h
  exact ⟨rfl, SS.of_eq (boot_info_sched hk1)⟩

/-! ## The dispatch -/

theorem matches_self (b : arch.bound.Bound NT NR NP NL NS NE) :
    Matches b b.cfg b.kernel.sched := by
  refine ⟨rfl, ?_⟩
  have : (taskOf : logic.TaskCfg → logic.TaskCfg) = id := by funext x; rfl
  rw [this, List.map_id]

/-- A dispatch of an eligible Task: its Priority is above the Job on top,
and it goes on top of the stack. -/
theorem push_ok {s1 : logic.sched.Sched NT NR NL} {t : U8} (hok : okB b)
    (hel : Elig b.cfg b.kernel.sched t.val)
    (hst : logic.sched.Sched.start b.kernel.sched b.cfg t = ok s1)
    (hc : b'.cfg = b.cfg) (hs : SS b'.kernel.sched s1) :
    okB b' ∧ (kvB b').prio = (kvB b).prio ∧ (kvB b').part = (kvB b).part ∧
    (∀ i, i < (kvB b).depth → (kvB b').stack i = (kvB b).stack i) ∧
    ((kvB b').depth = (kvB b).depth ∨
      ((kvB b').depth = (kvB b).depth + 1 ∧ (kvB b').stack (kvB b).depth = t.val)) := by
  have hpr : (kvB b').prio = (kvB b).prio := by simp only [kvB, hc]
  have hpa : (kvB b').part = (kvB b).part := by simp only [kvB, hc]
  obtain ⟨ht, hdn, d, hd, rfl⟩ := start_ok hst
  have hstk : ∀ p, (kvB b').stack p =
      if p = (kvB b).depth then t.val else (kvB b).stack p := by
    intro p
    have := hs.2 p
    simp only [kvB]
    rw [this]
    simp only [stk]
    rw [get_set _ _ _ _ hdn]
    by_cases hp : p = b.kernel.sched.depth.val <;> simp [hp]
  have hdep : (kvB b').depth = (kvB b).depth + 1 := by
    have := hs.1
    simp only [kvB]
    rw [this]
    exact hd
  obtain ⟨h1, h2, h3, h4⟩ := hok
  have htop : ∀ i, i < (kvB b).depth → (kvB b).prio ((kvB b).stack i) < (kvB b).prio t.val := by
    intro i hi
    obtain ⟨_, _, _, _, he⟩ := hel
    rcases he with h0 | hlt
    · simp only [kvB] at hi; omega
    · have hlt' : (kvB b).prio ((kvB b).stack ((kvB b).depth - 1)) < (kvB b).prio t.val := hlt
      by_cases hie : i = (kvB b).depth - 1
      · rw [hie]; exact hlt'
      · exact Nat.lt_trans (h4 i ((kvB b).depth - 1) (by omega) (by omega)) hlt'
  refine ⟨⟨h1, fun u hu => ?_, fun i hi => ?_, fun i j hij hj => ?_⟩, hpr, hpa, fun i hi => ?_,
    Or.inr ⟨hdep, ?_⟩⟩
  · rw [hpr, hpa]; exact h2 u hu
  · rw [hstk]
    split
    · exact ht
    · exact h3 i (by omega)
  · rw [hpr, hstk i, hstk j, if_neg (by omega)]
    by_cases hje : j = (kvB b).depth
    · rw [if_pos hje]; exact htop i (by omega)
    · rw [if_neg hje]; exact h4 i j hij (by omega)
  · rw [hstk, if_neg (by omega)]
  · rw [hstk, if_pos rfl]

/-- No dispatch: the stack is as it was. -/
theorem nodisp {t : U8} (hc : b'.cfg = b.cfg) (hs : SS b'.kernel.sched b.kernel.sched)
    (hok : okB b) :
    okB b' ∧ (kvB b').prio = (kvB b).prio ∧ (kvB b').part = (kvB b).part ∧
    (∀ i, i < (kvB b).depth → (kvB b').stack i = (kvB b).stack i) ∧
    ((kvB b').depth = (kvB b).depth ∨
      ((kvB b').depth = (kvB b).depth + 1 ∧ (kvB b').stack (kvB b).depth = t.val)) := by
  have hbs : BS b' b := ⟨hc, hs⟩
  obtain ⟨c1, c2⟩ := hbs.bp.kr hok
  exact ⟨c1, c2.2.2.1, c2.2.2.2, fun i _ => hbs.2.2 i, Or.inl hbs.depth⟩

set_option maxHeartbeats 2000000 in
theorem bound_dispatch {t : U8} {pc : U32} (hok : okB b)
    (h : arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch b t pc = ok b') :
    okB b' ∧ (kvB b').prio = (kvB b).prio ∧ (kvB b').part = (kvB b).part ∧
    (∀ i, i < (kvB b).depth → (kvB b').stack i = (kvB b).stack i) ∧
    ((kvB b').depth = (kvB b).depth ∨
      ((kvB b').depth = (kvB b).depth + 1 ∧ (kvB b').stack (kvB b).depth = t.val)) := by
  unfold arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch at h
  peel h
  all_goals first
    | exact push_ok hok ((eligible_ok (matches_self b) (by assumption)).mp (by assumption))
        (dispatch_eff (by assumption)) rfl (SS.of_eq (settle_sched (by assumption)))
    | exact nodisp rfl (SS.of_eq (settle_sched (by assumption))) hok

/-! ## The contract -/

/-- `ki` is the Kernel's instance of the trait: its methods the entry
cores' invariant depends on are the extracted ones. -/
structure IsBound (ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)) : Prop where
  start : ki.start = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.start
  job_info : ki.job_info = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_info
  depth : ki.depth = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.depth
  dispatch : ki.dispatch = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.dispatch
  job_return : ki.job_return = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.job_return
  svc_lock : ki.svc_lock = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_lock
  svc_unlock : ki.svc_unlock = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.svc_unlock
  settle : ki.settle = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.settle
  intake : ki.intake = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.intake
  timer_event : ki.timer_event = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.timer_event
  fault : ki.fault = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault
  take_reinit : ki.take_reinit = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_reinit
  take_stop_dma :
    ki.take_stop_dma = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.take_stop_dma
  record_event :
    ki.record_event = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.record_event
  fault_of : ki.fault_of = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.fault_of
  top : ki.top = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.top
  boot_failed : ki.boot_failed = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_failed
  boot_info : ki.boot_info = arch.bound.Bound.Insts.Rsk_kernelArchBoundKernelOps.boot_info

/-- **The Kernel that is linked satisfies the contract the entry cores are
proven against.** -/
theorem bound_kspec {ki : arch.bound.KernelOps (arch.bound.Bound NT NR NP NL NS NE)}
    (hb : IsBound ki) : KSpecD ki kvB okB where
  kok := okB_kok
  depth := fun h => by rw [hb.depth] at h; exact bound_depth h
  top := fun hok h => by rw [hb.top] at h; exact bound_top hok h
  job_info := fun _ h => by rw [hb.job_info] at h; exact bound_job_info h
  dispatch := fun hok h => by rw [hb.dispatch] at h; exact bound_dispatch hok h
  start := fun h => by rw [hb.start] at h; exact (bound_start_bs h).bp.kr
  job_return := fun h => by rw [hb.job_return] at h; exact (bound_job_return_bp h).1.kr
  job_return_depth := fun _ h hp => by
    rw [hb.job_return] at h; exact (bound_job_return_bp h).2 hp
  svc_lock := fun h => by rw [hb.svc_lock] at h; exact (bound_svc_lock_bs h).bp.kr
  svc_unlock := fun h => by rw [hb.svc_unlock] at h; exact (bound_svc_unlock_bs h).bp.kr
  settle := fun h => by rw [hb.settle] at h; exact (bound_settle_bs h).bp.kr
  settle_depth := fun _ h => by rw [hb.settle] at h; exact (bound_settle_bs h).depth
  intake := fun h => by rw [hb.intake] at h; exact (bound_intake_bp h).kr
  timer_event := fun h => by rw [hb.timer_event] at h; exact (bound_timer_event_bp h).kr
  fault := fun h => by rw [hb.fault] at h; exact (bound_fault_bp h).kr
  take_reinit := fun h => by rw [hb.take_reinit] at h; exact (bound_take_reinit_bs h).bp.kr
  take_reinit_depth := fun _ h => by
    rw [hb.take_reinit] at h; exact (bound_take_reinit_bs h).depth
  take_stop_dma := fun h => by
    rw [hb.take_stop_dma] at h; exact (bound_take_stop_dma_bs h).bp.kr
  take_stop_dma_depth := fun _ h => by
    rw [hb.take_stop_dma] at h; exact (bound_take_stop_dma_bs h).depth
  boot_failed := fun h => by rw [hb.boot_failed] at h; exact (bound_boot_failed_bs h).bp.kr
  boot_info := fun h => by rw [hb.boot_info] at h; exact (bound_boot_info_bs h).bp.kr
  record_event := fun h => by rw [hb.record_event] at h; exact (bound_record_event_bs h).bp.kr
  fault_of := fun h => by rw [hb.fault_of] at h; exact (bound_fault_of_bp h).kr
  svc_lock_depth := fun _ h => by rw [hb.svc_lock] at h; exact (bound_svc_lock_bs h).depth
  svc_unlock_depth := fun _ h => by
    rw [hb.svc_unlock] at h; exact (bound_svc_unlock_bs h).depth
  start_depth := fun _ h => by rw [hb.start] at h; exact (bound_start_bs h).depth
  fault_kernel_depth := fun _ h => by
    rw [hb.fault] at h; exact (bound_fault_kernel_bs h).depth

end P18
