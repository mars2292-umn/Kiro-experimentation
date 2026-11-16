import Proto.MHead
/-!
# The relation between a role and the runtime type

A role that a pending choice does not involve has, as its local type, the
merge of the branches' projections; once the choice is taken the runtime
type keeps the taken branch alone while the role's type is unchanged. So
the invariant does not project the runtime type: it relates the role's
local type to the runtime type by `Fits`, a coinductive relation (a
relation closed under `NodeOk`). Port of the sections "the relation" and
"the initial relation" of `crates/rsk-protocol/src/multiparty.rs`.
-/

namespace MP

def LabelsEq (ls : Ls) (bs : RBs) : Prop :=
  ∀ label, (ls.lookup label).isSome ↔ (bs.lookup label).isSome

/-- The conditions on one pair of a consistent relation: the runtime type
is well formed; a role with an action is reached on every path of it; and
at its head the role's next action is the head's, with the continuation
pairs in the relation, or, at a choice the role is not part of, the same
local type is paired with every branch. -/
def NodeOk (rel : L → R → Prop) (m : L) (x : R) (u n : Nat) : Prop :=
  x.wf n = true ∧ ((mhead m).acting = true → x.reaches u = true) ∧
  match x.head with
  | .fin => mhead m = .fin
  | .comm frm dst bs =>
    if u = frm then
      ∃ ls, mhead m = .send dst ls ∧ LabelsEq ls bs ∧
        ∀ label b, bs.lookup label = some b → ∃ c, ls.lookup label = some c ∧ rel c b
    else if u = dst then
      ∃ ls, mhead m = .recv frm ls ∧
        ∀ label b, bs.lookup label = some b → ∃ c, ls.lookup label = some c ∧ rel c b
    else ∀ label b, bs.lookup label = some b → rel m b
  | .sent frm dst label cont =>
    if u = dst then ∃ ls c, mhead m = .recv frm ls ∧ ls.lookup label = some c ∧ rel c cont
    else rel m cont
  | _ => False

def Consistent (rel : L → R → Prop) (u n : Nat) : Prop :=
  ∀ m x, rel m x → NodeOk rel m x u n

/-- The local type `m` of role `u` fits the runtime type `x`. -/
def Fits (m : L) (x : R) (u n : Nat) : Prop :=
  ∃ rel : L → R → Prop, Consistent rel u n ∧ rel m x

/-! ## The initial relation -/

/-- The relation of the initial configuration: the role's head refines the
head of the merged projection. -/
def Rel0 (u n : Nat) (m : L) (x : R) : Prop :=
  x.wf n = true ∧ ∃ l, rproj x u = some l ∧ Refines (mhead m) (mhead l)

theorem acting_refines {hm a : L} (h : Refines hm a) (ha : hm.acting = true) : a.acting = true := by
  cases hm with
  | send t ml => obtain ⟨al, rfl, _⟩ := refines_send_left.mp h; rfl
  | recv f ml => obtain ⟨al, rfl, _⟩ := refines_recv_left.mp h; rfl
  | fin => simp [L.acting] at ha
  | var v => simp [L.acting] at ha
  | mu b => simp [L.acting] at ha

theorem rel0_node {u n : Nat} {m : L} {x : R} (h : Rel0 u n m x) : NodeOk (Rel0 u n) m x u n := by
  obtain ⟨hw, l, hl, href⟩ := h
  have hhw := R.head_wf hw
  obtain ⟨lh, hlh, hmh⟩ := mhead_rproj _ x n u l rfl hw hl
  have href' : Refines (mhead m) (mhead lh) := by rw [hmh]; exact href
  refine ⟨hw, ?_, ?_⟩
  · intro hact
    apply action_reaches x u l hl
    cases ha : l.actionless with
    | false => rfl
    | true =>
      have hacta := acting_refines href hact
      rcases actionless_mhead ha with e | e <;> rw [e] at hacta <;> simp [L.acting, stuck] at hacta
  · cases hx : x.head with
    | fin =>
      rw [hx] at hlh
      simp only [rproj] at hlh
      cases hlh
      exact refines_fin_right href'
    | var v => exact absurd hx (hhw.2.2 v)
    | mu b => exact absurd hx (hhw.2.1 b)
    | comm frm dst bs =>
      rw [hx] at hlh hhw
      have hparts := R.comm_wf_parts hhw.1
      dsimp only
      rcases rproj_comm hlh with ⟨hu, ls, hls, rfl⟩ | ⟨hu1, hu, ls, hls, rfl⟩ | ⟨hu1, hu2, hm⟩
      · rw [if_pos hu]
        rw [show mhead (L.send dst ls) = L.send dst ls from rfl] at href'
        obtain ⟨hls', he, hsame, hrl⟩ := refines_send_right href'
        have hlook := rprojBs_lookup bs u ls hls
        refine ⟨hls', he, ?_, ?_⟩
        · intro label
          rw [hsame label]
          cases hb : bs.lookup label with
          | none => rw [hlook.2 label hb]; rfl
          | some b =>
            obtain ⟨lb, _, hl2⟩ := hlook.1 label b hb
            rw [hl2]; rfl
        · intro label b hb
          obtain ⟨lb, hlb, hl2⟩ := hlook.1 label b hb
          obtain ⟨c, hc, hcr⟩ := hrl label lb hl2
          exact ⟨c, hc, RBs.branch_wf bs n label b hparts.2.2.2.2.2 hb, lb, hlb, refines_mhead hcr⟩
      · rw [if_neg hu1, if_pos hu]
        rw [show mhead (L.recv frm ls) = L.recv frm ls from rfl] at href'
        obtain ⟨hls', he, hrl⟩ := refines_recv_right href'
        have hlook := rprojBs_lookup bs u ls hls
        refine ⟨hls', he, ?_⟩
        intro label b hb
        obtain ⟨lb, hlb, hl2⟩ := hlook.1 label b hb
        obtain ⟨c, hc, hcr⟩ := hrl label lb hl2
        exact ⟨c, hc, RBs.branch_wf bs n label b hparts.2.2.2.2.2 hb, lb, hlb, refines_mhead hcr⟩
      · rw [if_neg hu1, if_neg hu2]
        intro label b hb
        obtain ⟨lb, hlb, hr⟩ := rprojMerge_refines bs u lh hm label b hb
        exact ⟨RBs.branch_wf bs n label b hparts.2.2.2.2.2 hb, lb, hlb,
          href'.trans (refines_mhead hr)⟩
    | sent frm dst label cont =>
      rw [hx] at hlh hhw
      have hparts := R.sent_wf_parts hhw.1
      dsimp only
      obtain ⟨lc, hlc, hcase⟩ := rproj_sent hlh
      rcases hcase with ⟨hu, rfl⟩ | ⟨hu, rfl⟩
      · rw [if_pos hu]
        rw [show mhead (L.recv frm (Ls.cons label lc Ls.nil)) = L.recv frm (Ls.cons label lc Ls.nil) from rfl] at href'
        obtain ⟨hls', he, hrl⟩ := refines_recv_right href'
        obtain ⟨c, hc, hcr⟩ := hrl label lc (by simp [Ls.lookup])
        exact ⟨hls', c, he, hc, hparts.2.2.2, lc, hlc, refines_mhead hcr⟩
      · rw [if_neg hu]
        exact ⟨hparts.2.2.2, lh, hlc, href'⟩

theorem rel0_consistent (u n : Nat) : Consistent (Rel0 u n) u n :=
  fun _ _ h => rel0_node h

end MP
