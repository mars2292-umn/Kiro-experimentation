import Sched.TotalResp
import Sched.LoopInv
open Aeneas Aeneas.Std Result

/-! # The responses leave the instants of the time arithmetic alone (2026-11-20)

The time arithmetic of the timer and intake entries reads three arrays of
`Timing`: the nominal release instants, the instants of the last accepted
Releases, and the instants deferred Releases are held to. Its additions do
not overflow because those stay within bounds (`Sched/TotalTime.lean`).
This file shows that the responses and the events that apply them do not
write the three arrays (`TF`), so the bounds survive them. -/

namespace P18
open rsk_kernel rsk_kernel.rsk_logic

variable {NT NR NL NP NS NE : Usize}

/-- The three arrays of instants are those of `tm`. -/
def TFt (tm' tm : logic.time.Timing NT) : Prop :=
  tm'.nominal = tm.nominal ∧ tm'.last_due = tm.last_due ∧
  tm'.deferred_until = tm.deferred_until

/-- The three arrays of instants of `k'` are those of `k`. -/
def TF (k' k : logic.kernel.Kernel NT NR NP NL NS NE) : Prop := TFt k'.timing k.timing

theorem TF.refl (k : logic.kernel.Kernel NT NR NP NL NS NE) : TF k k := ⟨rfl, rfl, rfl⟩

theorem TF.trans {a b c : logic.kernel.Kernel NT NR NP NL NS NE} (h1 : TF b a) (h2 : TF c b) :
    TF c a := ⟨h2.1.trans h1.1, h2.2.1.trans h1.2.1, h2.2.2.trans h1.2.2⟩

syntax "tf_call" : tactic
macro_rules | `(tactic| tf_call) => `(tactic| fail "no call")

/-- Registers a lemma `l : f .. = ok (.., x') → (three equations on x' and x)`. -/
macro "tf_reg " l:ident : command =>
  `(macro_rules | `(tactic| tf_call) => `(tactic| first
      | refine Eq.trans (And.left ($l (by assumption))) ?_
      | refine Eq.trans (And.left (And.right ($l (by assumption)))) ?_
      | refine Eq.trans (And.right (And.right ($l (by assumption)))) ?_))

syntax "tf_proj" : tactic
macro_rules
  | `(tactic| tf_proj) => `(tactic| repeat (first
      | rfl
      | exact (And.left (by assumption : TF _ _))
      | exact (And.left (And.right (by assumption : TF _ _)))
      | exact (And.right (And.right (by assumption : TF _ _)))
      | tf_call
      | dsimp only))

/-- Closes `TF k' k` after `peel`. -/
syntax "tf_close" : tactic
macro_rules
  | `(tactic| tf_close) => `(tactic| (refine ⟨?_, ?_, ?_⟩ <;> (tf_proj; done)))

/-- `peel`, with the conditionals it leaves split by hand, each exit closed
by `tf_close`. -/
macro "tf_peel " h:ident : tactic =>
  `(tactic| (
      peel $h
      all_goals first
        | (tf_close; done)
        | (rcases ite_cases $h with ⟨_, $h⟩ | ⟨_, $h⟩ <;> peel $h <;> first
            | (tf_close; done)
            | (rcases ite_cases $h with ⟨_, $h⟩ | ⟨_, $h⟩ <;> peel $h <;> first
                | (tf_close; done)
                | (rcases ite_cases $h with ⟨_, $h⟩ | ⟨_, $h⟩ <;> peel $h <;>
                    (tf_close; done))))))

variable {k k' : logic.kernel.Kernel NT NR NP NL NS NE} {cfg : logic.Config NT NR NP}

theorem push_event_tf {kind task partition : U8} {prs : U64} {detail : U32}
    (h : logic.kernel.Kernel.push_event k cfg kind task partition prs detail = ok k') :
    TF k' k := by
  unfold logic.kernel.Kernel.push_event at h
  tf_peel h

tf_reg push_event_tf

theorem observe_tf {now : U64} (h : logic.kernel.Kernel.observe k cfg now = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.observe at h
  tf_peel h

tf_reg observe_tf

theorem try_unmask_tft {tm tm' : logic.time.Timing NT} {t : U8} {now : U64} {je b : Bool}
    (h : logic.time.Timing.try_unmask tm cfg t now je = ok (b, tm')) : TFt tm' tm := by
  unfold logic.time.Timing.try_unmask at h
  peel h
  all_goals exact ⟨rfl, rfl, rfl⟩

tf_reg try_unmask_tft

theorem arm_budget_tf {t : U8} (h : logic.kernel.Kernel.arm_budget k cfg t = ok k') :
    TF k' k := by
  unfold logic.kernel.Kernel.arm_budget at h
  tf_peel h

tf_reg arm_budget_tf

theorem end_mit_window_tf {t : U8} (h : logic.kernel.Kernel.end_mit_window k cfg t = ok k') :
    TF k' k := by
  unfold logic.kernel.Kernel.end_mit_window at h
  tf_peel h

tf_reg end_mit_window_tf

theorem enter_safe_state_tf (h : logic.kernel.Kernel.enter_safe_state k cfg = ok k') :
    TF k' k := by
  unfold logic.kernel.Kernel.enter_safe_state at h
  tf_peel h

tf_reg enter_safe_state_tf

theorem kill_at_tf {i : Usize} (h : logic.kernel.Kernel.kill_at k cfg i = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.kill_at at h
  tf_peel h

tf_reg kill_at_tf

theorem kill_at_timing {i : Usize} (h : logic.kernel.Kernel.kill_at k cfg i = ok k') :
    k'.timing = k.timing := by
  unfold logic.kernel.Kernel.kill_at at h
  peel h
  rfl

theorem kill_partition_jobs_tf {partition : U8}
    (h : logic.kernel.Kernel.kill_partition_jobs k cfg partition = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.kill_partition_jobs at h
  simp only [bind_eq_ok] at h
  obtain ⟨r, hr, h⟩ := h
  unfold logic.kernel.Kernel.kill_partition_jobs_loop at hr
  have key : r.2.2.2.2.2.2.2.2.2.1 = k.timing := by
    refine loop_inv _ (fun x => x.1.timing = k.timing) (fun r => r.2.2.2.2.2.2.2.2.2.1 = k.timing)
      ?_ ?_ (k, 0#usize) r rfl hr
    · rintro ⟨k1, i1⟩ ⟨k2, i2⟩ hinv hb
      dsimp only at hinv hb ⊢
      unfold logic.kernel.Kernel.kill_partition_jobs_loop.body at hb
      peel hb
      all_goals first
        | exact hinv
        | exact (kill_at_timing (by assumption)).trans hinv
    · rintro ⟨k1, i1⟩ y hinv hb
      dsimp only at hinv hb ⊢
      unfold logic.kernel.Kernel.kill_partition_jobs_loop.body at hb
      peel hb
      exact hinv
  obtain ⟨a, a1, a2, i, a3, i1, a4, a5, i2, t, s, b, l, a6, a7, a8, a9, b1, b2, i3, g⟩ := r
  dsimp only at key h
  have h := ok_inj h
  subst h
  subst key
  exact ⟨rfl, rfl, rfl⟩

tf_reg kill_partition_jobs_tf

set_option maxHeartbeats 2000000 in
theorem clear_loop_tf {partition : U8} {t : Usize}
    (h : logic.kernel.Kernel.clear_partition_releases_loop k cfg partition t = ok k') :
    TF k' k := by
  unfold logic.kernel.Kernel.clear_partition_releases_loop at h
  refine loop_inv _ (fun x => TF x.1 k) (fun r => TF r k) ?_ ?_ (k, t) k' (TF.refl k) h
  · rintro ⟨k1, i1⟩ ⟨k2, i2⟩ hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.clear_partition_releases_loop.body at hb
    tf_peel hb
  · rintro ⟨k1, i1⟩ y hinv hb
    dsimp only at hinv hb ⊢
    unfold logic.kernel.Kernel.clear_partition_releases_loop.body at hb
    tf_peel hb

tf_reg clear_loop_tf

theorem clear_partition_releases_tf {partition : U8}
    (h : logic.kernel.Kernel.clear_partition_releases k cfg partition = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.clear_partition_releases at h
  tf_peel h

tf_reg clear_partition_releases_tf

set_option maxHeartbeats 2000000 in
theorem end_top_release_tf (h : logic.kernel.Kernel.end_top_release k cfg = ok k') :
    TF k' k := by
  unfold logic.kernel.Kernel.end_top_release at h
  tf_peel h

tf_reg end_top_release_tf

theorem end_top_tf (h : logic.kernel.Kernel.end_top k cfg = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.end_top at h
  tf_peel h

tf_reg end_top_tf

theorem end_partition_jobs_tf {partition : U8}
    (h : logic.kernel.Kernel.end_partition_jobs k cfg partition = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.end_partition_jobs at h
  tf_peel h

tf_reg end_partition_jobs_tf

theorem stop_partition_tf {partition cause : U8}
    (h : logic.kernel.Kernel.stop_partition k cfg partition cause = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.stop_partition at h
  tf_peel h

tf_reg stop_partition_tf

theorem restart_partition_tf {partition cause : U8}
    (h : logic.kernel.Kernel.restart_partition k cfg partition cause = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.restart_partition at h
  tf_peel h

tf_reg restart_partition_tf

set_option maxHeartbeats 4000000 in
theorem apply_response_tf {partition response cause t : U8}
    (h : logic.kernel.Kernel.apply_response k cfg partition response cause t = ok k') :
    TF k' k := by
  unfold logic.kernel.Kernel.apply_response at h
  tf_peel h

tf_reg apply_response_tf

theorem mit_violation_tf {t partition offender : U8}
    (h : logic.kernel.Kernel.mit_violation k cfg t partition offender = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.mit_violation at h
  tf_peel h

tf_reg mit_violation_tf

theorem deadline_miss_tf {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.deadline_miss k cfg t i = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.deadline_miss at h
  tf_peel h

tf_reg deadline_miss_tf

set_option maxHeartbeats 2000000 in
theorem budget_event_tf {t : U8} {i : Usize}
    (h : logic.kernel.Kernel.budget_event k cfg t i = ok k') : TF k' k := by
  unfold logic.kernel.Kernel.budget_event at h
  tf_peel h

tf_reg budget_event_tf

end P18
