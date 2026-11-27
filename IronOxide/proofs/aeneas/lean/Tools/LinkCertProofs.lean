/-
  The Link_Checker's stack-usage certificate on the code as compiled
  (2026-11-26): `cert::check` of `rsk-link`, as Aeneas extracts it
  (`Tools/Link.lean`), returns for every input and returns `true` exactly
  when `Certified` holds (`check_spec`). Verus proves the direction from
  `true` (`crates/rsk-link/src/cert.rs`, `check`'s `ensures`); the
  direction to `true` is proven here only. `Certified` restates
  `cert::certified` (`WalkOk`, `CutOk`, `NodeOk` restate `walk_ok`,
  `cut_ok`, `node_ok`); that the two texts say the same is by reading.

  `walk_within_usage` and `one_panic_within_total` are the two theorems
  Verus proves from the certificate (`theorem_walk_within_usage`,
  `theorem_one_panic_within_total`), proven again here, and
  `check_walk_within_usage` joins the first to the checker: when the
  compiled `check` answers `true`, every chain of followed calls from a
  bounded function takes no more stack than the usage reported for it.

  Not covered, as in Verus: that the nodes handed to `check` are the
  walk's result (`graph::CallGraph::certificate`), the frame of each
  function and its callees (the disassembly), and the per-region
  composition of `stack::bounds`. Handwritten.
-/
import Aeneas
import Tools.Link
import Tools.FlagLoop
open Aeneas Aeneas.Std Result

namespace rsk_link
open cert

instance : Inhabited Node :=
  ⟨⟨0#u32, 0#u32, false, false, alloc.vec.Vec.new _, alloc.vec.Vec.new _⟩⟩

/-- `walk_ok` with the caller given as a node. -/
def WalkOkN (g : List Node) (node : Node) (c : Nat) : Prop :=
  c < g.length ∧ g[c]!.bounded = true ∧ node.frame.val + g[c]!.own.val ≤ node.own.val ∧
  (g[c]!.panics = true → node.panics = true)

/-- `cut_ok` with the caller given as a node. -/
def CutOkN (g : List Node) (node : Node) (c pu : Nat) : Prop :=
  c < g.length ∧ node.panics = true ∧ (g[c]!.bounded = true → g[c]!.own.val ≤ pu)

/-- `cert::walk_ok`: a followed call from `i` to `c`. -/
def WalkOk (g : List Node) (i c : Nat) : Prop := WalkOkN g g[i]! c

/-- `cert::cut_ok`: a panic edge from `i` to `c`. -/
def CutOk (g : List Node) (i c pu : Nat) : Prop := CutOkN g g[i]! c pu

/-- `cert::node_ok`. -/
def NodeOk (g : List Node) (i pu : Nat) : Prop :=
  g[i]!.bounded = true →
    g[i]!.frame.val ≤ g[i]!.own.val ∧
    (∀ k, k < g[i]!.walked.val.length → WalkOk g i (g[i]!.walked.val[k]!).val) ∧
    (∀ k, k < g[i]!.cut.val.length → CutOk g i (g[i]!.cut.val[k]!).val pu)

/-- `cert::certified`: the certificate. -/
def Certified (g : List Node) (pu : Nat) : Prop := ∀ i, i < g.length → NodeOk g i pu

theorem walked_loop_spec (g : alloc.vec.Vec Node) (node : Node) (n m : Usize)
    (hn : n.val = g.val.length) (hm : m.val = node.walked.val.length) :
    walked_check_loop g n node.frame node.own node.panics node.walked m true 0#usize
      ⦃ b => b = true ↔ ∀ k, k < m.val → WalkOkN g.val node (node.walked.val[k]!).val ⦄ := by
  unfold walked_check_loop
  apply FlagLoop.flag_loop _ m.val (fun k => WalkOkN g.val node (node.walked.val[k]!).val)
  · intro ok1 k h
    dsimp only
    unfold walked_check_loop.body
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro k hk _
    dsimp only
    unfold walked_check_loop.body
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    have hkl : k.val < node.walked.val.length := by omega
    step as ⟨ c, hc ⟩
    have ec : c = node.walked.val[k.val]! := by
      rw [hc]; exact (getElem!_pos node.walked.val k.val hkl).symm
    split
    · rename_i hge
      simp only [WP.spec_ok]
      left
      refine ⟨trivial, fun hw => ?_⟩
      obtain ⟨h1, _, _, _⟩ := hw
      rw [← ec] at h1
      scalar_tac
    · rename_i hlt
      have hcl : c.val < g.val.length := by scalar_tac
      step as ⟨ callee, hcal ⟩
      have ecal : callee = g.val[c.val]! := by
        rw [hcal]; exact (getElem!_pos g.val c.val hcl).symm
      split
      · rename_i hb
        have c1 : (UScalar.cast .U64 node.frame).val = node.frame.val := U32.cast_U64_val_eq _
        have c2 : (UScalar.cast .U64 callee.own).val = callee.own.val := U32.cast_U64_val_eq _
        have c3 : (UScalar.cast .U64 node.own).val = node.own.val := U32.cast_U64_val_eq _
        step as ⟨ i2, hi2 ⟩
        step as ⟨ i3, hi3 ⟩
        have d2 : i2.val = node.frame.val := by rw [hi2]; exact c1
        have d3 : i3.val = callee.own.val := by rw [hi3]; exact c2
        step as ⟨ i4, hi4 ⟩
        step as ⟨ i5, hi5 ⟩
        have d5 : i5.val = node.own.val := by rw [hi5]; exact c3
        split
        · rename_i hgt
          simp only [WP.spec_ok]
          left
          refine ⟨trivial, fun hw => ?_⟩
          obtain ⟨_, _, h3, _⟩ := hw
          rw [← ec, ← ecal] at h3
          scalar_tac
        · rename_i hle
          have hsum : node.frame.val + callee.own.val ≤ node.own.val := by scalar_tac
          split
          · rename_i hp
            split
            · rename_i hnp
              step as ⟨ k1, hk1 ⟩
              right
              refine ⟨k1, rfl, hk1, ?_⟩
              rw [← ec]
              refine ⟨hcl, ?_, ?_, ?_⟩
              · rw [← ecal]; exact hb
              · rw [← ecal]; exact hsum
              · intro _; exact hnp
            · rename_i hnp
              simp only [WP.spec_ok]
              left
              refine ⟨trivial, fun hw => ?_⟩
              obtain ⟨_, _, _, h4⟩ := hw
              rw [← ec, ← ecal] at h4
              exact hnp (h4 hp)
          · rename_i hp
            step as ⟨ k1, hk1 ⟩
            right
            refine ⟨k1, rfl, hk1, ?_⟩
            rw [← ec]
            refine ⟨hcl, ?_, ?_, ?_⟩
            · rw [← ecal]; exact hb
            · rw [← ecal]; exact hsum
            · intro hc2; rw [← ecal] at hc2; exact absurd hc2 hp
      · rename_i hb
        simp only [WP.spec_ok]
        left
        refine ⟨trivial, fun hw => ?_⟩
        obtain ⟨_, h2, _, _⟩ := hw
        rw [← ec, ← ecal] at h2
        exact hb h2

theorem cut_loop_spec (g : alloc.vec.Vec Node) (node : Node) (pu : U32) (n m : Usize)
    (hn : n.val = g.val.length) (hm : m.val = node.cut.val.length) :
    cut_check_loop g pu n node.panics node.cut m true 0#usize
      ⦃ b => b = true ↔ ∀ k, k < m.val → CutOkN g.val node (node.cut.val[k]!).val pu.val ⦄ := by
  unfold cut_check_loop
  apply FlagLoop.flag_loop _ m.val (fun k => CutOkN g.val node (node.cut.val[k]!).val pu.val)
  · intro ok1 k h
    dsimp only
    unfold cut_check_loop.body
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro k hk _
    dsimp only
    unfold cut_check_loop.body
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    have hkl : k.val < node.cut.val.length := by omega
    step as ⟨ c, hc ⟩
    have ec : c = node.cut.val[k.val]! := by
      rw [hc]; exact (getElem!_pos node.cut.val k.val hkl).symm
    split
    · rename_i hge
      simp only [WP.spec_ok]
      left
      refine ⟨trivial, fun hw => ?_⟩
      obtain ⟨h1, _, _⟩ := hw
      rw [← ec] at h1
      scalar_tac
    · rename_i hlt
      have hcl : c.val < g.val.length := by scalar_tac
      split
      · rename_i hnp
        step as ⟨ callee, hcal ⟩
        have ecal : callee = g.val[c.val]! := by
          rw [hcal]; exact (getElem!_pos g.val c.val hcl).symm
        split
        · rename_i hb
          split
          · rename_i hgt
            simp only [WP.spec_ok]
            left
            refine ⟨trivial, fun hw => ?_⟩
            obtain ⟨_, _, h3⟩ := hw
            rw [← ec, ← ecal] at h3
            have := h3 hb
            scalar_tac
          · rename_i hle
            step as ⟨ k1, hk1 ⟩
            right
            refine ⟨k1, rfl, hk1, ?_⟩
            rw [← ec]
            refine ⟨hcl, hnp, ?_⟩
            intro _
            rw [← ecal]
            scalar_tac
        · rename_i hb
          step as ⟨ k1, hk1 ⟩
          right
          refine ⟨k1, rfl, hk1, ?_⟩
          rw [← ec]
          refine ⟨hcl, hnp, ?_⟩
          intro hc2
          rw [← ecal] at hc2
          exact absurd hc2 hb
      · rename_i hnp
        simp only [WP.spec_ok]
        left
        refine ⟨trivial, fun hw => ?_⟩
        obtain ⟨_, h2, _⟩ := hw
        exact hnp h2

theorem walked_check_spec (g : alloc.vec.Vec Node) (i : Usize) (hi : i.val < g.val.length) :
    walked_check g i ⦃ b => b = true ↔
      ∀ k, k < g.val[i.val]!.walked.val.length →
        WalkOk g.val i.val (g.val[i.val]!.walked.val[k]!).val ⦄ := by
  unfold walked_check
  step as ⟨ node, hnode ⟩
  have en : node = g.val[i.val]! := by
    rw [hnode]; exact (getElem!_pos g.val i.val hi).symm
  subst en
  exact walked_loop_spec g _ _ _ rfl rfl

theorem cut_check_spec (g : alloc.vec.Vec Node) (i : Usize) (pu : U32)
    (hi : i.val < g.val.length) :
    cut_check g i pu ⦃ b => b = true ↔
      ∀ k, k < g.val[i.val]!.cut.val.length →
        CutOk g.val i.val (g.val[i.val]!.cut.val[k]!).val pu.val ⦄ := by
  unfold cut_check
  step as ⟨ node, hnode ⟩
  have en : node = g.val[i.val]! := by
    rw [hnode]; exact (getElem!_pos g.val i.val hi).symm
  subst en
  exact cut_loop_spec g _ pu _ _ rfl rfl

theorem node_check_spec (g : alloc.vec.Vec Node) (i : Usize) (pu : U32)
    (hi : i.val < g.val.length) :
    node_check g i pu ⦃ b => b = true ↔ NodeOk g.val i.val pu.val ⦄ := by
  unfold node_check
  step as ⟨ node, hnode ⟩
  have en : node = g.val[i.val]! := by
    rw [hnode]; exact (getElem!_pos g.val i.val hi).symm
  split
  · rename_i hb
    split
    · rename_i hgt
      simp only [WP.spec_ok]
      constructor
      · intro hc; cases hc
      · intro hok
        have h1 := (hok (by rw [← en]; exact hb)).1
        rw [← en] at h1
        scalar_tac
    · rename_i hle
      step with (walked_check_spec g i hi) as ⟨ b, hbw ⟩
      split
      · rename_i hbt
        refine WP.spec_mono (cut_check_spec g i pu hi) ?_
        intro r hr
        constructor
        · intro hrt _
          refine ⟨?_, hbw.1 hbt, hr.1 hrt⟩
          rw [← en]; scalar_tac
        · intro hok
          exact hr.2 (hok (by rw [← en]; exact hb)).2.2
      · rename_i hbf
        simp only [WP.spec_ok]
        constructor
        · intro hc; cases hc
        · intro hok
          exact absurd (hbw.2 (hok (by rw [← en]; exact hb)).2.1) hbf
  · rename_i hb
    simp only [WP.spec_ok]
    constructor
    · intro _ hc
      rw [← en] at hc
      exact absurd hc hb
    · intro _; trivial

theorem check_loop_spec (g : alloc.vec.Vec Node) (pu : U32) (n : Usize)
    (hn : n.val = g.val.length) :
    check_loop g pu n true 0#usize ⦃ b => b = true ↔ Certified g.val pu.val ⦄ := by
  unfold check_loop
  refine WP.spec_mono
    (FlagLoop.flag_loop _ n.val (fun i => NodeOk g.val i pu.val) ?_ ?_) ?_
  · intro ok1 i h
    dsimp only
    unfold check_loop.body
    rcases h with h | h
    · rw [if_neg (by scalar_tac)]
    · subst h; split <;> rfl
  · intro i hi _
    dsimp only
    unfold check_loop.body
    rw [if_pos (by scalar_tac)]
    simp only [if_true]
    step with (node_check_spec g i pu (by omega)) as ⟨ b, hb ⟩
    split
    · rename_i hbt
      step as ⟨ i1, hi1 ⟩
      right
      exact ⟨i1, rfl, hi1, hb.1 hbt⟩
    · rename_i hbf
      simp only [WP.spec_ok]
      left
      exact ⟨trivial, fun hc => hbf (hb.2 hc)⟩
  · intro b hb
    rw [hn] at hb
    exact hb

/-- **The certificate checker.** `cert::check` as compiled returns for
every graph and every charge of the panic path, and answers `true` exactly
when the certificate holds. -/
theorem check_spec (g : alloc.vec.Vec Node) (pu : U32) :
    check g pu ⦃ b => b = true ↔ Certified g.val pu.val ⦄ := by
  unfold check
  exact check_loop_spec g pu _ rfl

/-! ## What the certificate gives (the two theorems of `cert.rs`) -/

/-- Function `i` has a followed call to function `c` (`cert::follows`). -/
def Follows (g : List Node) (i c : Nat) : Prop :=
  ∃ k, k < g[i]!.walked.val.length ∧ (g[i]!.walked.val[k]!).val = c

/-- Function `i` has a panic edge to function `c` (`cert::cuts`). -/
def Cuts (g : List Node) (i c : Nat) : Prop :=
  ∃ k, k < g[i]!.cut.val.length ∧ (g[i]!.cut.val[k]!).val = c

/-- A chain of followed calls from `a`: each function calls the next
(`cert::walk_path`, the chain as its first function and the rest). -/
def WalkPath (g : List Node) : Nat → List Nat → Prop
  | a, [] => a < g.length
  | a, b :: p => a < g.length ∧ Follows g a b ∧ WalkPath g b p

/-- The stack bytes a chain of calls holds at its deepest point: the sum
of its functions' frames (`cert::path_bytes`). -/
def pathBytes (g : List Node) (p : List Nat) : Nat := (p.map fun i => g[i]!.frame.val).sum

/-- R37.1 (`cert::theorem_walk_within_usage`): under the certificate,
every chain of followed calls from a bounded function takes no more stack
than the usage reported for that function; its last function is bounded;
and if its last function can panic, the first is marked so. -/
theorem walk_within_usage (g : List Node) (pu : Nat) (hc : Certified g pu) :
    ∀ (p : List Nat) (a : Nat), WalkPath g a p → g[a]!.bounded = true →
      pathBytes g (a :: p) ≤ g[a]!.own.val ∧
      g[(a :: p).getLast (List.cons_ne_nil a p)]!.bounded = true ∧
      (g[(a :: p).getLast (List.cons_ne_nil a p)]!.panics = true → g[a]!.panics = true) := by
  intro p
  induction p with
  | nil =>
    intro a ha hb
    have hn := hc a ha hb
    refine ⟨?_, hb, fun h => h⟩
    simp only [pathBytes, List.map_cons, List.map_nil, List.sum_cons, List.sum_nil, Nat.add_zero]
    exact hn.1
  | cons b p ih =>
    intro a ha hb
    obtain ⟨hal, ⟨k, hk, hkb⟩, hrest⟩ := ha
    have hn := hc a hal hb
    have hw : WalkOk g a b := by
      have := hn.2.1 k hk
      rw [hkb] at this
      exact this
    obtain ⟨hbl, hbb, hsum, hpan⟩ := hw
    obtain ⟨i1, i2, i3⟩ := ih b hrest hbb
    refine ⟨?_, ?_, ?_⟩
    · simp only [pathBytes, List.map_cons, List.sum_cons] at i1 ⊢
      omega
    · rw [List.getLast_cons (List.cons_ne_nil b p)]
      exact i2
    · rw [List.getLast_cons (List.cons_ne_nil b p)]
      intro h
      exact hpan (i3 h)

/-- R37.1 with the panic path (`cert::theorem_one_panic_within_total`): a
chain of followed calls from a bounded function, then one panic edge, then
a chain of followed calls inside the panic path takes no more stack than
the function's usage plus the panic path's charge, and the function is
marked as able to panic. -/
theorem one_panic_within_total (g : List Node) (pu : Nat) (hc : Certified g pu)
    (a : Nat) (p1 : List Nat) (b : Nat) (p2 : List Nat)
    (h1 : WalkPath g a p1) (hb : g[a]!.bounded = true) (h2 : WalkPath g b p2)
    (hcut : Cuts g ((a :: p1).getLast (List.cons_ne_nil a p1)) b)
    (hbb : g[b]!.bounded = true) :
    pathBytes g (a :: p1) + pathBytes g (b :: p2) ≤ g[a]!.own.val + pu ∧
    g[a]!.panics = true := by
  obtain ⟨w1, w2, w3⟩ := walk_within_usage g pu hc p1 a h1 hb
  obtain ⟨v1, _, _⟩ := walk_within_usage g pu hc p2 b h2 hbb
  have hlast : (a :: p1).getLast (List.cons_ne_nil a p1) < g.length := by
    clear w1 w2 w3 hcut hb
    induction p1 generalizing a with
    | nil => exact h1
    | cons c p ih =>
      obtain ⟨_, _, h1r⟩ := h1
      rw [List.getLast_cons (List.cons_ne_nil c p)]
      exact ih c h1r
  obtain ⟨k, hk, hkb⟩ := hcut
  have hn := hc _ hlast w2
  have hcu := hn.2.2 k hk
  rw [hkb] at hcu
  obtain ⟨_, hp, hown⟩ := hcu
  have := hown hbb
  exact ⟨by omega, w3 hp⟩

/-- **The checker and the bound.** When the compiled `check` answers
`true` for a graph, every chain of followed calls from a bounded function
of it takes no more stack than the usage the walk reported for that
function. -/
theorem check_walk_within_usage (g : alloc.vec.Vec Node) (pu : U32)
    (h : check g pu = ok true) (a : Nat) (p : List Nat) (hp : WalkPath g.val a p)
    (hb : g.val[a]!.bounded = true) :
    pathBytes g.val (a :: p) ≤ g.val[a]!.own.val := by
  have hs := check_spec g pu
  rw [h] at hs
  have hcert : Certified g.val pu.val := by simpa [WP.spec_ok] using hs
  exact (walk_within_usage g.val pu.val hcert p a hp hb).1

end rsk_link
