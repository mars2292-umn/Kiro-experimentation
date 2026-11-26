/-
  The Generator's acceptance conditions on the code as compiled
  (2026-11-26): `accept` of `rsk_gen::accept_v`, as Aeneas extracts it
  (`Tools/GenAccept.lean`), returns for every input and returns `true`
  exactly when `Accepted` holds (`accept_spec`), the statement Verus
  proves of it. The Generator emits nothing when `accept` answers no, so
  no Generated_Config is produced for a declaration outside the
  conditions. `Accepted` restates `accept_v::accepted`; that the two
  texts say the same is by reading. Handwritten.

  Not covered, as in Verus: the rules about names and paths, the
  Target's peripherals and interrupts, the Protocol form's per-label
  validation functions, and that `image` (not verified) hands `accept`
  what `derive` produced.
-/
import Aeneas
import Tools.GenAccept
import Tools.FlagLoop
open Aeneas Aeneas.Std Result

namespace rsk_gen
open accept_v

instance : Inhabited TaskA := ⟨⟨0#u8, 0#usize, 0#u64, 0#u64, 0#u32⟩⟩

instance : Inhabited EndpointA := ⟨⟨false, 0#u32, alloc.vec.Vec.new _, alloc.vec.Vec.new _⟩⟩
instance : Inhabited PpiA := ⟨⟨0#u8, 0#u32, 0#usize, 0#u32, 0#usize, false, 0#u32, 0#usize⟩⟩

def TaskOk (t : TaskA) (parts : Nat) : Prop :=
  1 ≤ t.priority.val ∧ t.priority.val ≤ 7 ∧ t.partition.val < parts ∧ 1 ≤ t.period.val ∧
  1 ≤ t.deadline.val ∧ t.deadline.val ≤ t.period.val ∧ 1 ≤ t.budget.val

def Upward (tasks : List TaskA) (levels : List U8) (r f : Nat) : Prop :=
  tasks[r]!.partition ≠ tasks[f]!.partition ∧
  (levels[tasks[r]!.partition.val]!).val < (levels[tasks[f]!.partition.val]!).val

def RoleOk (tasks : List TaskA) (e : EndpointA) (r : Nat) : Prop :=
  (e.roles.val[r]!).val < tasks.length ∧ (e.validates.val[r]!).val.length = e.roles.val.length

def EdgeOk (tasks : List TaskA) (levels : List U8) (e : EndpointA) (r f : Nat) : Prop :=
  ((e.validates.val[r]!).val[f]! = true →
    Upward tasks levels (e.roles.val[r]!).val (e.roles.val[f]!).val) ∧
  (e.plain = true → r ≠ f → Upward tasks levels (e.roles.val[r]!).val (e.roles.val[f]!).val →
    (e.validates.val[r]!).val[f]! = true)

/-- The decision the loop makes on one edge, as one condition. -/
theorem edge_ite {α} (v3 up plain rf : Bool) (good bad : α) :
    (if v3 = true then
        if up = true then
          if plain = true then
            if rf = true then
              if up = true then
                if v3 = true then good else bad
              else good
            else good
          else good
        else bad
      else
        if plain = true then
          if rf = true then
            if up = true then
              if v3 = true then good else bad
            else good
          else good
        else good) =
    if (v3 = true ∧ up = false) ∨ (plain = true ∧ rf = true ∧ up = true ∧ v3 = false)
    then bad else good := by
  cases v3 <;> cases up <;> cases plain <;> cases rf <;> simp

theorem edge_iff (v3 up plain rf : Bool) (UP RF : Prop) (hu : up = true ↔ UP)
    (hr : rf = true ↔ RF) :
    ((v3 = true → UP) ∧ (plain = true → RF → UP → v3 = true)) ↔
    ¬ ((v3 = true ∧ up = false) ∨ (plain = true ∧ rf = true ∧ up = true ∧ v3 = false)) := by
  rw [← hu, ← hr]
  cases v3 <;> cases up <;> cases plain <;> cases rf <;> simp

theorem edges_loop_spec (tasks : alloc.vec.Vec TaskA) (levels : alloc.vec.Vec U8) (e : EndpointA)
    (r : Usize) (hr : r.val < e.roles.val.length)
    (hv : e.validates.val.length = e.roles.val.length)
    (hroles : ∀ m, m < e.roles.val.length → RoleOk tasks.val e m)
    (htasks : ∀ i, i < tasks.val.length → TaskOk tasks.val[i]! levels.val.length)
    (n : Usize) (hn : n.val = e.roles.val.length) :
    edges_accept_loop tasks levels e.plain e.roles e.validates r n true 0#usize
      ⦃ b => b = true ↔ ∀ f, f < e.roles.val.length → EdgeOk tasks.val levels.val e r.val f ⦄ := by
  unfold edges_accept_loop
  apply loop.spec_decr_nat
    (fun (s : Bool × Usize) => 2 * (n.val - s.2.val) + (if s.1 then 1 else 0))
    (fun s => s.2.val ≤ n.val ∧ (∀ f, f < s.2.val → EdgeOk tasks.val levels.val e r.val f) ∧
      (s.1 = false → s.2.val < n.val ∧ ¬ EdgeOk tasks.val levels.val e r.val s.2.val))
  · intro s hs
    obtain ⟨ok1, f⟩ := s
    obtain ⟨h1, h2, h3⟩ := hs
    dsimp only at h1 h2 h3 ⊢
    unfold edges_accept_loop.body
    split
    · rename_i hlt
      split
      · rename_i hok
        have hf : f.val < e.roles.val.length := by scalar_tac
        have hrr := hroles r.val hr
        have hrf := hroles f.val hf
        step as ⟨ tr, htr ⟩
        step as ⟨ tf, htf ⟩
        have etr : tr = e.roles.val[r.val]! := by rw [htr]; exact (getElem!_pos e.roles.val r.val hr).symm
        have etf : tf = e.roles.val[f.val]! := by rw [htf]; exact (getElem!_pos e.roles.val f.val hf).symm
        have htrl : tr.val < tasks.val.length := by rw [etr]; exact hrr.1
        have htfl : tf.val < tasks.val.length := by rw [etf]; exact hrf.1
        step as ⟨ ta, hta ⟩
        step as ⟨ ta1, hta1 ⟩
        have eta : ta = tasks.val[tr.val]! := by rw [hta]; exact (getElem!_pos tasks.val tr.val htrl).symm
        have eta1 : ta1 = tasks.val[tf.val]! := by rw [hta1]; exact (getElem!_pos tasks.val tf.val htfl).symm
        have tok := htasks tr.val htrl
        have tok1 := htasks tf.val htfl
        rw [← eta] at tok
        rw [← eta1] at tok1
        have hv2l : r.val < e.validates.val.length := by omega
        have hrowl : (e.validates.val[r.val]!).val.length = e.roles.val.length := hrr.2
        split
        case' isTrue hne =>
          have hpl : ta.partition.val < levels.val.length := tok.2.2.1
          have hpl1 : ta1.partition.val < levels.val.length := tok1.2.2.1
          step as ⟨ l0, hl0 ⟩
          step as ⟨ l1, hl1 ⟩
          have hup : decide (l0 < l1) = true ↔ Upward tasks.val levels.val tr.val tf.val := by
            unfold Upward
            rw [← eta, ← eta1]
            have el0 : l0 = levels.val[ta.partition.val]! := by
              rw [hl0]; exact (getElem!_pos levels.val _ hpl).symm
            have el1 : l1 = levels.val[ta1.partition.val]! := by
              rw [hl1]; exact (getElem!_pos levels.val _ hpl1).symm
            rw [← el0, ← el1]
            simp only [decide_eq_true_eq]
            constructor
            · intro h; exact ⟨by simpa using hne, h⟩
            · intro h; exact h.2
        case' isFalse heq =>
          have hup : false = true ↔ Upward tasks.val levels.val tr.val tf.val := by
            unfold Upward
            rw [← eta, ← eta1]
            constructor
            · intro h; cases h
            · intro h; exact absurd h.1 (by simpa using heq)
        all_goals
          step as ⟨ v2, hv2 ⟩
          have ev2 : v2 = e.validates.val[r.val]! := by
            rw [hv2]; exact (getElem!_pos e.validates.val r.val hv2l).symm
          have hfl : f.val < v2.val.length := by rw [ev2, hrowl]; exact hf
          step as ⟨ v3, hv3 ⟩
          have ev3 : v3 = (e.validates.val[r.val]!).val[f.val]! := by
            rw [hv3]; subst ev2; exact (getElem!_pos _ f.val hfl).symm
          have hrfb : (r != f) = true ↔ r.val ≠ f.val := by
            simp only [bne_iff_ne, ne_eq]
            constructor
            · intro h hc; exact h (by scalar_tac)
            · intro h hc; exact h (by rw [hc])
          have hE0 := edge_iff v3 _ e.plain (r != f) _ _ hup hrfb
          have hE := Iff.trans (show EdgeOk tasks.val levels.val e r.val f.val ↔
              ((v3 = true → Upward tasks.val levels.val tr.val tf.val) ∧
                (e.plain = true → r.val ≠ f.val → Upward tasks.val levels.val tr.val tf.val →
                  v3 = true)) from by unfold EdgeOk; rw [← ev3, ← etr, ← etf]) hE0
          rw [edge_ite]
          split
          · rename_i hbad
            simp only [WP.spec_ok]
            refine ⟨h1, h2, fun _ => ⟨by scalar_tac, fun hc => (hE.1 hc) hbad⟩, ?_⟩
            simp
          · rename_i hgood
            step as ⟨ f1, hf1 ⟩
            refine ⟨by scalar_tac, ?_, ?_, ?_⟩
            · intro g hg
              by_cases hgf : g = f.val
              · rw [hgf]; exact hE.2 hgood
              · exact h2 g (by omega)
            · intro hc; rw [hok] at hc; cases hc
            · simp only [hok, if_true]
              scalar_tac
      · rename_i hnok
        simp only [WP.spec_ok]
        have hfalse : ok1 = false := by simpa using hnok
        obtain ⟨hfn, hbad⟩ := h3 hfalse
        rw [hfalse]
        constructor
        · intro hc; cases hc
        · intro hall; exact absurd (hall f.val (by omega)) hbad
    · rename_i hge
      simp only [WP.spec_ok]
      have hfn : f.val = n.val := by scalar_tac
      constructor
      · intro _ g hg; exact h2 g (by omega)
      · intro hall
        cases hk : ok1 with
        | true => rfl
        | false => exact absurd (h3 hk).1 (by omega)
  · refine ⟨by scalar_tac, fun g hg => absurd hg (by simp), fun hc => by cases hc⟩

/-- R42.4 on every edge from role `r` (`edges_accept`'s contract). -/
theorem edges_accept_spec (tasks : alloc.vec.Vec TaskA) (levels : alloc.vec.Vec U8) (e : EndpointA)
    (r : Usize) (hr : r.val < e.roles.val.length)
    (hv : e.validates.val.length = e.roles.val.length)
    (hroles : ∀ m, m < e.roles.val.length → RoleOk tasks.val e m)
    (htasks : ∀ i, i < tasks.val.length → TaskOk tasks.val[i]! levels.val.length) :
    edges_accept tasks levels e r
      ⦃ b => b = true ↔ ∀ f, f < e.roles.val.length → EdgeOk tasks.val levels.val e r.val f ⦄ := by
  unfold edges_accept
  exact edges_loop_spec tasks levels e r hr hv hroles htasks _ rfl

theorem role_loop0_spec (tasks : alloc.vec.Vec TaskA) (e : EndpointA) (n : Usize)
    (hn : n.val = e.roles.val.length) (hv : e.validates.val.length = e.roles.val.length) :
    endpoint_accept_loop0 tasks e.roles e.validates n true 0#usize
      ⦃ b => b = true ↔ ∀ m, m < n.val → RoleOk tasks.val e m ⦄ := by
  unfold endpoint_accept_loop0
  apply FlagLoop.flag_loop _ n.val (fun m => RoleOk tasks.val e m)
  · intro ok1 i h
    dsimp only
    unfold endpoint_accept_loop0.body
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold endpoint_accept_loop0.body
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    have hil : i.val < e.roles.val.length := by omega
    step as ⟨ t, ht ⟩
    have et : t = e.roles.val[i.val]! := by rw [ht]; exact (getElem!_pos e.roles.val i.val hil).symm
    simp only [alloc.vec.Vec.len]
    split
    · rename_i hge
      simp only [WP.spec_ok]
      left
      refine ⟨trivial, fun hc => ?_⟩
      have := hc.1
      rw [← et] at this
      scalar_tac
    · rename_i hlt
      step as ⟨ v2, hv2 ⟩
      have ev2 : v2 = e.validates.val[i.val]! := by
        rw [hv2]; exact (getElem!_pos e.validates.val i.val (by omega)).symm
      split
      · rename_i hne
        simp only [WP.spec_ok]
        left
        refine ⟨trivial, fun hc => ?_⟩
        have := hc.2
        rw [← ev2] at this
        have hne' : v2.val.length ≠ n.val := by
          intro hc2
          simp only [bne_iff_ne, ne_eq] at hne
          exact hne (by scalar_tac)
        omega
      · rename_i heq
        step as ⟨ i1, hi1 ⟩
        right
        refine ⟨i1, rfl, hi1, ?_, ?_⟩
        · rw [← et]; scalar_tac
        · rw [← ev2]
          have : v2.val.length = n.val := by simpa using heq
          omega

theorem role_loop2_spec (tasks : alloc.vec.Vec TaskA) (e : EndpointA) (n : Usize)
    (hn : n.val = e.roles.val.length) (hv : e.validates.val.length = e.roles.val.length) :
    endpoint_accept_loop2 tasks e.roles e.validates n true 0#usize
      ⦃ b => b = true ↔ ∀ m, m < n.val → RoleOk tasks.val e m ⦄ := by
  unfold endpoint_accept_loop2
  apply FlagLoop.flag_loop _ n.val (fun m => RoleOk tasks.val e m)
  · intro ok1 i h
    dsimp only
    unfold endpoint_accept_loop2.body
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold endpoint_accept_loop2.body
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    have hil : i.val < e.roles.val.length := by omega
    step as ⟨ t, ht ⟩
    have et : t = e.roles.val[i.val]! := by rw [ht]; exact (getElem!_pos e.roles.val i.val hil).symm
    simp only [alloc.vec.Vec.len]
    split
    · rename_i hge
      simp only [WP.spec_ok]
      left
      refine ⟨trivial, fun hc => ?_⟩
      have := hc.1
      rw [← et] at this
      scalar_tac
    · rename_i hlt
      step as ⟨ v2, hv2 ⟩
      have ev2 : v2 = e.validates.val[i.val]! := by
        rw [hv2]; exact (getElem!_pos e.validates.val i.val (by omega)).symm
      split
      · rename_i hne
        simp only [WP.spec_ok]
        left
        refine ⟨trivial, fun hc => ?_⟩
        have := hc.2
        rw [← ev2] at this
        have hne' : v2.val.length ≠ n.val := by
          intro hc2
          simp only [bne_iff_ne, ne_eq] at hne
          exact hne (by scalar_tac)
        omega
      · rename_i heq
        step as ⟨ i1, hi1 ⟩
        right
        refine ⟨i1, rfl, hi1, ?_, ?_⟩
        · rw [← et]; scalar_tac
        · rw [← ev2]
          have : v2.val.length = n.val := by simpa using heq
          omega

theorem row_loop1_spec (tasks : alloc.vec.Vec TaskA) (levels : alloc.vec.Vec U8) (e : EndpointA)
    (n : Usize) (hn : n.val = e.roles.val.length)
    (hv : e.validates.val.length = e.roles.val.length)
    (hroles : ∀ m, m < e.roles.val.length → RoleOk tasks.val e m)
    (htasks : ∀ i, i < tasks.val.length → TaskOk tasks.val[i]! levels.val.length) :
    endpoint_accept_loop1 tasks levels e.plain e.capacity e.roles e.validates n true 0#usize
      ⦃ b => b = true ↔ ∀ r, r < n.val → ∀ f, f < e.roles.val.length →
        EdgeOk tasks.val levels.val e r f ⦄ := by
  unfold endpoint_accept_loop1
  apply FlagLoop.flag_loop _ n.val
    (fun r => ∀ f, f < e.roles.val.length → EdgeOk tasks.val levels.val e r f)
  · intro ok1 i h
    dsimp only
    unfold endpoint_accept_loop1.body
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold endpoint_accept_loop1.body
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    step with (edges_accept_spec tasks levels e i (by omega) hv hroles htasks) as ⟨ b1, hb1 ⟩
    split
    · rename_i hb
      step as ⟨ i1, hi1 ⟩
      right
      exact ⟨i1, rfl, hi1, hb1.1 hb⟩
    · rename_i hb
      simp only [WP.spec_ok]
      left
      exact ⟨trivial, fun hc => hb (hb1.2 hc)⟩

theorem row_loop3_spec (tasks : alloc.vec.Vec TaskA) (levels : alloc.vec.Vec U8) (e : EndpointA)
    (n : Usize) (hn : n.val = e.roles.val.length)
    (hv : e.validates.val.length = e.roles.val.length)
    (hroles : ∀ m, m < e.roles.val.length → RoleOk tasks.val e m)
    (htasks : ∀ i, i < tasks.val.length → TaskOk tasks.val[i]! levels.val.length) :
    endpoint_accept_loop3 tasks levels e.plain e.capacity e.roles e.validates n true 0#usize
      ⦃ b => b = true ↔ ∀ r, r < n.val → ∀ f, f < e.roles.val.length →
        EdgeOk tasks.val levels.val e r f ⦄ := by
  unfold endpoint_accept_loop3
  apply FlagLoop.flag_loop _ n.val
    (fun r => ∀ f, f < e.roles.val.length → EdgeOk tasks.val levels.val e r f)
  · intro ok1 i h
    dsimp only
    unfold endpoint_accept_loop3.body
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold endpoint_accept_loop3.body
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    step with (edges_accept_spec tasks levels e i (by omega) hv hroles htasks) as ⟨ b1, hb1 ⟩
    split
    · rename_i hb
      step as ⟨ i1, hi1 ⟩
      right
      exact ⟨i1, rfl, hi1, hb1.1 hb⟩
    · rename_i hb
      simp only [WP.spec_ok]
      left
      exact ⟨trivial, fun hc => hb (hb1.2 hc)⟩

def EndpointOk (tasks : List TaskA) (levels : List U8) (e : EndpointA) : Prop :=
  2 ≤ e.roles.val.length ∧ e.roles.val.length ≤ 4 ∧ (e.plain = true → e.roles.val.length = 2) ∧
  1 ≤ e.capacity.val ∧ e.validates.val.length = e.roles.val.length ∧
  (∀ r, r < e.roles.val.length → RoleOk tasks e r) ∧
  (∀ r, r < e.roles.val.length → ∀ f, f < e.roles.val.length → EdgeOk tasks levels e r f)

/-- **One Endpoint** (`endpoint_accept`'s contract): with every Task
accepted, the function returns and decides `EndpointOk`. -/
theorem endpoint_accept_spec (tasks : alloc.vec.Vec TaskA) (levels : alloc.vec.Vec U8)
    (e : EndpointA)
    (htasks : ∀ i, i < tasks.val.length → TaskOk tasks.val[i]! levels.val.length) :
    endpoint_accept tasks levels e ⦃ b => b = true ↔ EndpointOk tasks.val levels.val e ⦄ := by
  unfold endpoint_accept
  simp only [alloc.vec.Vec.len]
  have hlen : (Usize.ofNatCore e.roles.val.length (by scalar_tac)).val = e.roles.val.length := by
    scalar_tac
  generalize hnd : Usize.ofNatCore e.roles.val.length (by scalar_tac) = n at *
  unfold EndpointOk
  split
  · rename_i h; simp only [WP.spec_ok]
    constructor
    · intro hc; cases hc
    · intro hc; have := hc.1; scalar_tac
  split
  · rename_i _ h; simp only [WP.spec_ok]
    constructor
    · intro hc; cases hc
    · intro hc; have := hc.2.1; scalar_tac
  split
  · rename_i _ _ hplain
    split
    · rename_i h; simp only [WP.spec_ok]
      constructor
      · intro hc; cases hc
      · intro hc
        have := hc.2.2.1 hplain
        simp only [bne_iff_ne, ne_eq] at h
        exact absurd (by scalar_tac) h
    split
    · rename_i _ h; simp only [WP.spec_ok]
      constructor
      · intro hc; cases hc
      · intro hc; have := hc.2.2.2.1; scalar_tac
    split
    · rename_i _ _ h; simp only [WP.spec_ok]
      constructor
      · intro hc; cases hc
      · intro hc
        have := hc.2.2.2.2.1
        simp only [bne_iff_ne, ne_eq] at h
        exact absurd (by scalar_tac) h
    · rename_i h2 h4 hn2 hcap hvl
      have hv : e.validates.val.length = e.roles.val.length := by
        simp only [bne_iff_ne, ne_eq, not_not] at hvl
        scalar_tac
      step with (role_loop0_spec tasks e n hlen hv) as ⟨ ok1, hok1 ⟩
      split
      · rename_i hok
        have hroles : ∀ m, m < e.roles.val.length → RoleOk tasks.val e m := by
          intro m hm; exact hok1.1 hok m (by omega)
        subst hok
        refine WP.spec_mono (row_loop1_spec tasks levels e n hlen hv hroles htasks) ?_
        intro b hb
        rw [hb]
        constructor
        · intro hall
          refine ⟨by scalar_tac, by scalar_tac, fun _ => ?_, by scalar_tac, hv, hroles, ?_⟩
          · simp only [bne_iff_ne, ne_eq, not_not] at hn2; scalar_tac
          · intro r hr; exact hall r (by omega)
        · intro hc r hr; exact hc.2.2.2.2.2.2 r (by omega)
      · rename_i hok
        simp only [WP.spec_ok]
        constructor
        · intro hc; cases hc
        · intro hc
          exact absurd (hok1.2 (fun m hm => hc.2.2.2.2.2.1 m (by omega))) hok
  · rename_i _ _ hplain
    split
    · rename_i _ h; simp only [WP.spec_ok]
      constructor
      · intro hc; cases hc
      · intro hc; have := hc.2.2.2.1; scalar_tac
    split
    · rename_i _ _ h; simp only [WP.spec_ok]
      constructor
      · intro hc; cases hc
      · intro hc
        have := hc.2.2.2.2.1
        simp only [bne_iff_ne, ne_eq] at h
        exact absurd (by scalar_tac) h
    · rename_i h2 h4 hcap hvl
      have hv : e.validates.val.length = e.roles.val.length := by
        simp only [bne_iff_ne, ne_eq, not_not] at hvl
        scalar_tac
      step with (role_loop2_spec tasks e n hlen hv) as ⟨ ok1, hok1 ⟩
      split
      · rename_i hok
        have hroles : ∀ m, m < e.roles.val.length → RoleOk tasks.val e m := by
          intro m hm; exact hok1.1 hok m (by omega)
        subst hok
        refine WP.spec_mono (row_loop3_spec tasks levels e n hlen hv hroles htasks) ?_
        intro b hb
        rw [hb]
        constructor
        · intro hall
          refine ⟨by scalar_tac, by scalar_tac, fun hp => absurd hp hplain, by scalar_tac, hv,
            hroles, ?_⟩
          intro r hr; exact hall r (by omega)
        · intro hc r hr; exact hc.2.2.2.2.2.2 r (by omega)
      · rename_i hok
        simp only [WP.spec_ok]
        constructor
        · intro hc; cases hc
        · intro hc
          exact absurd (hok1.2 (fun m hm => hc.2.2.2.2.2.1 m (by omega))) hok

theorem tasks_loop_spec (tasks : alloc.vec.Vec TaskA) (levels : alloc.vec.Vec U8) :
    accept_loop0 tasks levels true 0#usize
      ⦃ b => b = true ↔ ∀ i, i < tasks.val.length → TaskOk tasks.val[i]! levels.val.length ⦄ := by
  unfold accept_loop0
  apply FlagLoop.flag_loop _ tasks.val.length (fun i => TaskOk tasks.val[i]! levels.val.length)
  · intro ok1 i h
    dsimp only
    unfold accept_loop0.body
    simp only [alloc.vec.Vec.len]
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold accept_loop0.body
    simp only [alloc.vec.Vec.len]
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    step as ⟨ t, ht ⟩
    have et : t = tasks.val[i.val]! := by rw [ht]; exact (getElem!_pos tasks.val i.val hi).symm
    rw [← et]
    unfold TaskOk
    repeat' split
    all_goals first
      | (step as ⟨ i3, hi3 ⟩
         right
         exact ⟨i3, rfl, hi3, by scalar_tac, by scalar_tac, by scalar_tac, by scalar_tac,
           by scalar_tac, by scalar_tac, by scalar_tac⟩)
      | (simp only [WP.spec_ok]
         left
         refine ⟨trivial, fun hc => ?_⟩
         obtain ⟨c1, c2, c3, c4, c5, c6, c7⟩ := hc
         scalar_tac)

theorem endpoints_loop_spec (tasks : alloc.vec.Vec TaskA) (levels : alloc.vec.Vec U8)
    (endpoints : alloc.vec.Vec EndpointA)
    (htasks : ∀ i, i < tasks.val.length → TaskOk tasks.val[i]! levels.val.length) :
    accept_loop1 tasks levels endpoints true 0#usize
      ⦃ b => b = true ↔ ∀ j, j < endpoints.val.length →
        EndpointOk tasks.val levels.val endpoints.val[j]! ⦄ := by
  unfold accept_loop1
  apply FlagLoop.flag_loop _ endpoints.val.length
    (fun j => EndpointOk tasks.val levels.val endpoints.val[j]!)
  · intro ok1 i h
    dsimp only
    unfold accept_loop1.body
    simp only [alloc.vec.Vec.len]
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold accept_loop1.body
    simp only [alloc.vec.Vec.len]
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    step as ⟨ ea, hea ⟩
    have eea : ea = endpoints.val[i.val]! := by
      rw [hea]; exact (getElem!_pos endpoints.val i.val hi).symm
    rw [← eea]
    step with (endpoint_accept_spec tasks levels ea htasks) as ⟨ b, hb ⟩
    split
    · rename_i hbt
      step as ⟨ j1, hj1 ⟩
      right
      exact ⟨j1, rfl, hj1, hb.1 hbt⟩
    · rename_i hbf
      simp only [WP.spec_ok]
      left
      exact ⟨trivial, fun hc => hbf (hb.2 hc)⟩

/-- An event register of a peripheral: a word from 0x100 to 0x1FC. -/
def EventReg (o : Nat) : Prop := 0x100 ≤ o ∧ o < 0x200 ∧ o % 4 = 0

/-- A task register of a peripheral: a word from 0x000 to 0x0FC. -/
def TaskReg (o : Nat) : Prop := o < 0x100 ∧ o % 4 = 0

/-- PM33.1 and PM33.2 on the channel at position `i`. -/
def PpiOk (c : PpiA) (i parts : Nat) : Prop :=
  c.channel.val = i ∧ EventReg c.event_offset.val ∧ TaskReg c.task_offset.val ∧
  c.event_owner.val < parts ∧ c.task_owner = c.event_owner ∧
  (c.fork = true → TaskReg c.fork_offset.val ∧ c.fork_owner = c.event_owner)

theorem lt_ofNatCore {x : Usize} {n : Nat} {hb : n < 2 ^ UScalarTy.Usize.numBits} :
    x < Usize.ofNatCore n hb ↔ x.val < n := by
  constructor <;> intro h <;> scalar_tac

theorem cast_eq_iff {ch i : Usize} {c8 : U8} (hch : ch = UScalar.cast UScalarTy.Usize c8) :
    ch = i ↔ c8.val = i.val := by
  subst hch
  constructor <;> intro h <;> scalar_tac

set_option maxHeartbeats 4000000 in
theorem ppi_loop_spec (levels : alloc.vec.Vec U8) (ppi : alloc.vec.Vec PpiA) :
    accept_loop2 levels ppi true 0#usize
      ⦃ b => b = true ↔ ∀ k, k < ppi.val.length → PpiOk ppi.val[k]! k levels.val.length ⦄ := by
  unfold accept_loop2
  apply FlagLoop.flag_loop _ ppi.val.length (fun k => PpiOk ppi.val[k]! k levels.val.length)
  · intro ok1 i h
    dsimp only
    unfold accept_loop2.body
    simp only [alloc.vec.Vec.len]
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold accept_loop2.body
    simp only [alloc.vec.Vec.len]
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    step as ⟨ c, hc ⟩
    have ec : c = ppi.val[i.val]! := by rw [hc]; exact (getElem!_pos ppi.val i.val hi).symm
    rw [← ec]
    have hreg : (if 256#u32 <= c.event_offset then
          if c.event_offset < 512#u32 then do
            let i1 ← c.event_offset % 4#u32
            if i1 = 0#u32 then
              if c.task_offset < 256#u32 then do
                let i2 ← c.task_offset % 4#u32
                ok (decide (i2 = 0#u32))
              else ok false
            else ok false
          else ok false
        else ok false : Result Bool)
        ⦃ b => b = true ↔ EventReg c.event_offset.val ∧ TaskReg c.task_offset.val ⦄ := by
      unfold EventReg TaskReg
      split
      · split
        · step as ⟨ i1, hi1 ⟩
          split
          · split
            · step as ⟨ i2, hi2 ⟩
              simp only [decide_eq_true_eq]
              constructor
              · intro h; scalar_tac
              · intro h; scalar_tac
            · simp only [WP.spec_ok]
              constructor
              · intro h; cases h
              · intro h; scalar_tac
          · simp only [WP.spec_ok]
            constructor
            · intro h; cases h
            · intro h; scalar_tac
        · simp only [WP.spec_ok]
          constructor
          · intro h; cases h
          · intro h; scalar_tac
      · simp only [WP.spec_ok]
        constructor
        · intro h; cases h
        · intro h; scalar_tac
    have hfork : (if c.fork then
          if c.fork_offset < 256#u32 then do
            let i1 ← c.fork_offset % 4#u32
            if i1 = 0#u32 then ok (decide (c.fork_owner = c.event_owner))
            else ok false
          else ok false
        else ok true : Result Bool)
        ⦃ b => b = true ↔
          (c.fork = true → TaskReg c.fork_offset.val ∧ c.fork_owner = c.event_owner) ⦄ := by
      unfold TaskReg
      split
      · rename_i hf
        split
        · step as ⟨ i1, hi1 ⟩
          split
          · simp only [WP.spec_ok, decide_eq_true_eq]
            constructor
            · intro h _; exact ⟨by scalar_tac, by scalar_tac, h⟩
            · intro h; exact (h hf).2.2
          · simp only [WP.spec_ok]
            constructor
            · intro h; cases h
            · intro h; have := (h hf).1; scalar_tac
        · simp only [WP.spec_ok]
          constructor
          · intro h; cases h
          · intro h; have := (h hf).1; scalar_tac
      · rename_i hf
        simp only [WP.spec_ok]
        constructor
        · intro _ h; exact absurd h hf
        · intro _; trivial
    step with hreg as ⟨ registers, hregs ⟩
    step with hfork as ⟨ fork, hfk ⟩
    step as ⟨ ch, hch ⟩
    clear hreg hfork
    unfold PpiOk
    split
    · rename_i h1
      split
      · rename_i h2
        split
        · rename_i h3
          split
          · rename_i h4
            split
            · rename_i h5
              step as ⟨ k1, hk1 ⟩
              right
              exact ⟨k1, rfl, hk1, (cast_eq_iff hch).1 h1, (hregs.1 h2).1, (hregs.1 h2).2,
                lt_ofNatCore.1 h3, h4, hfk.1 h5⟩
            · rename_i h5
              simp only [WP.spec_ok]
              left
              exact ⟨trivial, fun hcc => h5 (hfk.2 hcc.2.2.2.2.2)⟩
          · rename_i h4
            simp only [WP.spec_ok]
            left
            exact ⟨trivial, fun hcc => h4 hcc.2.2.2.2.1⟩
        · rename_i h3
          simp only [WP.spec_ok]
          left
          exact ⟨trivial, fun hcc => h3 (lt_ofNatCore.2 hcc.2.2.2.1)⟩
      · rename_i h2
        simp only [WP.spec_ok]
        left
        exact ⟨trivial, fun hcc => h2 (hregs.2 ⟨hcc.2.1, hcc.2.2.1⟩)⟩
    · rename_i h1
      simp only [WP.spec_ok]
      left
      exact ⟨trivial, fun hcc => h1 ((cast_eq_iff hch).2 hcc.1)⟩

/-- **The acceptance conditions** (`accept_v::accepted`). -/
def Accepted (tasks : List TaskA) (levels : List U8) (endpoints : List EndpointA)
    (ppi : List PpiA) (ppi_limit : Nat) : Prop :=
  1 ≤ levels.length ∧
  (∀ i, i < tasks.length → TaskOk tasks[i]! levels.length) ∧
  (∀ i, i < endpoints.length → EndpointOk tasks levels endpoints[i]!) ∧
  ppi.length ≤ ppi_limit ∧
  (∀ i, i < ppi.length → PpiOk ppi[i]! i levels.length)

/-- **`accept` decides the acceptance conditions** (`accept`'s contract):
the function returns for every input, and returns `true` exactly when
`Accepted` holds. So no Generated_Config is produced for a declaration
outside the conditions, whatever the validation code does. -/
theorem accept_spec (tasks : alloc.vec.Vec TaskA) (levels : alloc.vec.Vec U8)
    (endpoints : alloc.vec.Vec EndpointA) (ppi : alloc.vec.Vec PpiA) (ppi_limit : Usize) :
    accept tasks levels endpoints ppi ppi_limit
      ⦃ b => b = true ↔ Accepted tasks.val levels.val endpoints.val ppi.val ppi_limit.val ⦄ := by
  unfold accept Accepted
  simp only [alloc.vec.Vec.len]
  split
  · rename_i h
    simp only [WP.spec_ok]
    constructor
    · intro hc; cases hc
    · intro hc; have := hc.1; scalar_tac
  · rename_i hl
    step with (tasks_loop_spec tasks levels) as ⟨ ok1, hok1 ⟩
    split
    · rename_i hk1
      subst hk1
      have htasks := hok1.1 rfl
      step with (endpoints_loop_spec tasks levels endpoints htasks) as ⟨ ok2, hok2 ⟩
      split
      · rename_i hk2
        subst hk2
        have heps := hok2.1 rfl
        split
        · rename_i hgt
          simp only [WP.spec_ok]
          constructor
          · intro hc; cases hc
          · intro hc; have := hc.2.2.2.1; scalar_tac
        · rename_i hle
          refine WP.spec_mono (ppi_loop_spec levels ppi) ?_
          intro b hb
          rw [hb]
          constructor
          · intro hall; exact ⟨by scalar_tac, htasks, heps, by scalar_tac, hall⟩
          · intro hc; exact hc.2.2.2.2
      · rename_i hk2
        simp only [WP.spec_ok]
        constructor
        · intro hc; cases hc
        · intro hc; exact absurd (hok2.2 hc.2.2.1) hk2
    · rename_i hk1
      simp only [WP.spec_ok]
      constructor
      · intro hc; cases hc
      · intro hc; exact absurd (hok1.2 hc.2.1) hk1

end rsk_gen
