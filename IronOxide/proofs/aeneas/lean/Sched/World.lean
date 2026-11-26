import Sched.StubModel
import Sched.ArchRel
import Sched.ArchBound
open Aeneas Aeneas.Std Result

/-! # The main stack and the level records agree (2026-11-20)

The join of the two halves of Property 2: the machine model of the stubs
(`Sched/StubModel.lean`) and the entry cores as extracted, with the
invariant of `Sched/ArchInv.lean`. Verus: `world` and the stub theorems of
`stub_model.rs`.

`World lv mem base ws tpsp`: the register records `ws` lie on the main
stack from `base` upward, one per active level record, the topmost level
first; each has the shape its level record's EXC_RETURN value says and
holds that value as its saved LR; and every record but the oldest was
pushed over a Job, in Thread mode on the process stack, with nothing
between it and the next.

Each theorem takes a stub entered in a machine state `s`, its Rust half
answering as the extracted core does, and shows the agreement holds again
in the state the stub leaves:

- `kentry_world`: a Kernel-level handler (SVC, faults);
- `irq_stub_world`: `rsk_irq_entry`, on its three paths (a dispatch pushes
  the record of the new level, built from the entry's registers; a return
  as entered; a branch to `rsk_unwind`);
- `unwind_round_world`: one round of `rsk_unwind` pops exactly the record
  of the level `epilogue_core` unwound, and gives back the registers that
  record holds.

So the register record a dispatch pushes for its level stays where it is,
matched with that level's record, through every later entry, until the
round of `rsk_unwind` that clears the level's record pops it: what the two
earlier theorems (`unwind_returns_dispatched_context`,
`dispatch_unwind_restores`) took as hypotheses. -/

namespace P18
open rsk_kernel Stub

abbrev LA := Array arch.cm4.plan.LevelRecord 8#usize

/-- The EXC_RETURN value of level `l`'s record, as the machine holds it. -/
def excOf (lv : LA) (l : Nat) : Int := ((lv.val[l]!).exc_return.val : Int)

/-- The register record of a level has the shape its record's EXC_RETURN
value says, and holds that value as the saved LR. -/
def frOk (lv : LA) (x : Nat × Stub.Frame) : Prop :=
  x.2.hasFp = !hasBit (excOf lv x.1) 16 ∧ x.2.regs 9 = excOf lv x.1

/-- The level was dispatched over Thread mode on the process stack. -/
def thr (lv : LA) (l : Nat) : Prop :=
  hasBit (excOf lv l) 8 = true ∧ hasBit (excOf lv l) 4 = true

def WOk (lv : LA) : List (Nat × Stub.Frame) → Prop
  | [] => True
  | x :: rest => frOk lv x ∧ (rest ≠ [] → x.2.gap = 0 ∧ thr lv x.1) ∧ WOk lv rest

/-- The main stack and the level records agree. -/
structure World (lv : LA) (mem : Int → Int) (base : Int) (ws : List (Nat × Stub.Frame))
    (tpsp : Bool) : Prop where
  frames : framesAt mem base (ws.map Prod.snd)
  desc : ws.Pairwise (fun a b => b.1 < a.1)
  act : ∀ l, (lvOf lv).act l = true ↔ l ∈ ws.map Prod.fst
  ok : WOk lv ws
  tp : ws ≠ [] → tpsp = true

theorem WOk.congr {lv lv' : LA} : ∀ ws : List (Nat × Stub.Frame),
    (∀ x ∈ ws, lv'.val[x.1]! = lv.val[x.1]!) → WOk lv ws → WOk lv' ws := by
  intro ws
  induction ws with
  | nil => intro _ _; trivial
  | cons x rest ih =>
    intro h hw
    obtain ⟨a, b, c⟩ := hw
    have hx := h x (List.mem_cons_self)
    refine ⟨?_, fun hne => ?_, ih (fun y hy => h y (List.mem_cons_of_mem _ hy)) c⟩
    · simp only [frOk, excOf] at a ⊢
      rw [hx]; exact a
    · obtain ⟨g, t⟩ := b hne
      simp only [thr, excOf] at t ⊢
      rw [hx]; exact ⟨g, t⟩

theorem World.mem_ext {lv : LA} {m1 m2 : Int → Int} {base : Int} {ws : List (Nat × Stub.Frame)}
    {tp : Bool} (h : World lv m1 base ws tp) (hm : ∀ x, base ≤ x → m2 x = m1 x) :
    World lv m2 base ws tp :=
  ⟨framesAt_ext _ _ hm h.frames, h.desc, h.act, h.ok, h.tp⟩

theorem World.levels_eq {lv lv' : LA} {m : Int → Int} {base : Int} {ws : List (Nat × Stub.Frame)}
    {tp : Bool} (h : World lv m base ws tp) (e : lv' = lv) : World lv' m base ws tp := by
  subst e; exact h

theorem lv_act_iff (lv : LA) (l : Nat) :
    (lvOf lv).act l = true ↔ l < 8 ∧ (lv.val[l]!).active = true := by
  simp only [lvOf, Bool.and_eq_true, decide_eq_true_eq]

/-- A dispatch: one record written above every active one, its register
record pushed on the main stack. -/
theorem World.push {lv lv' : LA} {m m' : Int → Int} {base sp : Int} {ws : List (Nat × Stub.Frame)}
    {tp : Bool} {level : Nat} {f : Stub.Frame} {e : U32} (h : World lv m base ws tp)
    (h8 : level < 8) (hsame : ∀ l, l ≠ level → lv'.val[l]! = lv.val[l]!)
    (hex : (lv'.val[level]!).exc_return = e) (hact : (lv'.val[level]!).active = true)
    (hlow : ∀ l, (lvOf lv).act l = true → l < level)
    (hft : frameTop m' sp f) (hbase : base = sp + frameSize f + f.gap)
    (hm : ∀ x, base ≤ x → m' x = m x)
    (hfp : f.hasFp = !hasBit (e.val : Int) 16) (hr9 : f.regs 9 = (e.val : Int))
    (hup : ws ≠ [] → f.gap = 0 ∧ hasBit (e.val : Int) 8 = true ∧ hasBit (e.val : Int) 4 = true) :
    World lv' m' sp ((level, f) :: ws) true := by
  have hmem : ∀ x ∈ ws, x.1 < level := fun x hx =>
    hlow x.1 ((h.act x.1).mpr (List.mem_map_of_mem hx))
  have hexc : excOf lv' level = (e.val : Int) := by simp only [excOf]; rw [hex]
  refine ⟨?_, ?_, ?_, ?_, fun _ => rfl⟩
  · show frameTop m' sp f ∧ framesAt m' (sp + frameSize f + f.gap) (ws.map Prod.snd)
    exact ⟨hft, hbase ▸ framesAt_ext _ _ hm h.frames⟩
  · exact List.pairwise_cons.mpr ⟨fun b hb => hmem b hb, h.desc⟩
  · intro l
    simp only [List.map_cons, List.mem_cons]
    by_cases hl : l = level
    · subst hl
      exact ⟨fun _ => Or.inl rfl, fun _ => (lv_act_iff lv' l).mpr ⟨h8, hact⟩⟩
    · have he : (lvOf lv').act l = (lvOf lv).act l := by
        simp only [lvOf]; rw [hsame l hl]
      rw [he, h.act l]
      exact ⟨fun x => Or.inr x, fun x => x.resolve_left hl⟩
  · refine ⟨⟨?_, ?_⟩, fun hne => ?_, WOk.congr ws (fun x hx => hsame x.1 (by have := hmem x hx; omega)) h.ok⟩
    · show f.hasFp = !hasBit (excOf lv' level) 16
      rw [hexc]; exact hfp
    · show f.regs 9 = excOf lv' level
      rw [hexc]; exact hr9
    · obtain ⟨a, b, c⟩ := hup hne
      exact ⟨a, by simp only [thr]; rw [hexc]; exact ⟨b, c⟩⟩

/-- An unwind: the topmost record cleared, its register record popped. -/
theorem World.pop {lv lv' : LA} {m m' : Int → Int} {sp : Int} {rest : List (Nat × Stub.Frame)}
    {tp tp' : Bool} {lt : Nat} {f : Stub.Frame} (h : World lv m sp ((lt, f) :: rest) tp)
    (hsame : ∀ l, l ≠ lt → lv'.val[l]! = lv.val[l]!) (hoff : (lv'.val[lt]!).active = false)
    (hm : ∀ x, sp ≤ x → m' x = m x) (htp : rest ≠ [] → tp' = true) :
    World lv' m' (sp + frameSize f + f.gap) rest tp' := by
  obtain ⟨hft, hfr⟩ := h.frames
  have hg : 0 ≤ f.gap := hft.2.2
  have hsz := frameSize_pos f
  obtain ⟨hlt, hdesc⟩ := List.pairwise_cons.mp h.desc
  have hne : ∀ x ∈ rest, x.1 ≠ lt := fun x hx => by have := hlt x hx; omega
  refine ⟨framesAt_ext _ _ (fun x hx => hm x (by omega)) hfr, hdesc, fun l => ?_,
    WOk.congr rest (fun x hx => hsame x.1 (hne x hx)) h.ok.2.2, htp⟩
  constructor
  · intro ha
    have hl : l ≠ lt := by
      intro e
      rw [e] at ha
      have := ((lv_act_iff lv' lt).mp ha).2
      rw [hoff] at this; cases this
    have he : (lvOf lv').act l = (lvOf lv).act l := by
      simp only [lvOf]; rw [hsame l hl]
    rw [he, h.act l] at ha
    simp only [List.map_cons, List.mem_cons] at ha
    exact ha.resolve_left hl
  · intro hin
    obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hin
    have hl := hne x hx
    have he : (lvOf lv').act x.1 = (lvOf lv).act x.1 := by
      simp only [lvOf]; rw [hsame x.1 hl]
    rw [he, h.act x.1]
    simp only [List.map_cons, List.mem_cons]
    exact Or.inr hin

/-- The topmost register record is that of the topmost active level. -/
theorem World.top {lv : LA} {m : Int → Int} {sp : Int} {ws : List (Nat × Stub.Frame)} {tp : Bool}
    (h : World lv m sp ws tp) {lt : Nat} (htop : Arch.IsTop (lvOf lv) lt) :
    ∃ f rest, ws = (lt, f) :: rest := by
  have hin := (h.act lt).mp htop.1
  cases ws with
  | nil => simp at hin
  | cons x rest =>
    obtain ⟨l1, f⟩ := x
    have ha1 : (lvOf lv).act l1 = true := (h.act l1).mpr (by simp)
    have hle : l1 ≤ lt := by
      by_contra hgt
      have := htop.2 l1 (by omega)
      rw [ha1] at this; cases this
    simp only [List.map_cons, List.mem_cons] at hin
    rcases hin with e | hr
    · exact ⟨f, rest, by rw [e]⟩
    · obtain ⟨y, hy, hye⟩ := List.mem_map.mp hr
      have := (List.pairwise_cons.mp h.desc).1 y hy
      rw [hye] at this
      exact absurd this (by omega)

/-- An active record, so a register record on the main stack. -/
theorem World.nonempty {lv : LA} {m : Int → Int} {sp : Int} {ws : List (Nat × Stub.Frame)} {tp : Bool}
    (h : World lv m sp ws tp) {l : Nat} (ha : (lvOf lv).act l = true) : ws ≠ [] := by
  intro e
  have := (h.act l).mp ha
  rw [e] at this
  simp at this

/-! ## The exception model at a handler's entry -/

/-- HW-06, as `stub_model.rs` states it: LR is the EXC_RETURN value of the
interrupted context; taken from Thread mode (bit 3), bit 2 tells whether
that context is on the process stack, and then nothing was stacked on the
main stack (`g = 0`); a frame stacked on the main stack is at least a
basic frame. -/
def hwEntry (lr g : Int) (tpsp : Bool) : Prop :=
  0 ≤ g ∧ (hasBit lr 8 = true → (hasBit lr 4 = true ↔ tpsp = true)) ∧
  (hasBit lr 8 = true → hasBit lr 4 = true → g = 0) ∧ (hasBit lr 4 = false → 32 ≤ g)

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-- A plan to unwind, at an entry from Thread mode: a Job was executing, on
the process stack, and the entry stacked nothing on the main stack. -/
theorem plan_g0 {k : K} {s1 : arch.cm4.plan.ArchState} {hop : Nat} {m : Int → Int} {base : Int}
    {ws : List (Nat × Stub.Frame)} {tp : Bool} {lr g : Int} (hai : AI kv k s1 hop) (hhop : hop ≠ 0)
    (hw : World s1.levels m base ws tp) (he : hwEntry lr g tp) (h8 : hasBit lr 8 = true) :
    g = 0 := by
  obtain ⟨⟨l, ha, _⟩, _⟩ := hai.2.1 hhop
  have htp := hw.tp (hw.nonempty ha)
  exact he.2.2.1 h8 ((he.2.1 h8).mpr htp)

/-- The machine's test of a bit of a 32-bit value is the core's. -/
theorem hasBit_u32 (x : U32) (mask : Nat) : hasBit (x.val : Int) mask = (x.val &&& mask != 0) := by
  have : (x.val : Int).toNat % 4294967296 = x.val := by
    have := x.hBounds
    simp only [Int.toNat_natCast]
    scalar_tac
  simp only [hasBit, this]

theorem hasBit8 {x : U32} (h : x &&& 8#u32 ≠ 0#u32) : hasBit (x.val : Int) 8 = true := by
  rw [hasBit_u32]
  have hv : (x &&& 8#u32).val ≠ 0 := fun e => h (by scalar_tac)
  rw [UScalar.val_and] at hv
  simpa using hv

/-! ## The Kernel-level handlers -/

/-- A Kernel-level handler whose core left the level records alone
(`Frame`) and whose stub left the main stack at and above the entry's
records untouched: the agreement holds as it did. With a plan to unwind,
handed to an entry from Thread mode, the records start at the stack pointer
`rsk_unwind` is entered with. -/
theorem kentry_world {k' : K} {s0 s1 : arch.cm4.plan.ArchState} {s u : M} {g : Int}
    {ws : List (Nat × Stub.Frame)} {tp : Bool} {hop : Nat}
    (hw : World s0.levels s.mem (s.sp + g) ws tp) (he : hwEntry s.lr g tp)
    (hfr : Frame s1 s0) (hai : AI kv k' s1 hop)
    (hsp : u.sp = s.sp) (hmem : ∀ x, s.sp + g ≤ x → u.mem x = s.mem x)
    (h8 : hop ≠ 0 → hasBit s.lr 8 = true) :
    World s1.levels u.mem (s.sp + g) ws tp ∧ (hop ≠ 0 → World s1.levels u.mem u.sp ws tp) := by
  have hw1 : World s1.levels u.mem (s.sp + g) ws tp := (hw.mem_ext hmem).levels_eq hfr.1
  refine ⟨hw1, fun hh => ?_⟩
  have hg := plan_g0 hai hh hw1 he (h8 hh)
  rw [hsp]
  rw [hg] at hw1
  simpa using hw1

/-- `rsk_busfault_entry` and `rsk_usagefault_entry`'s stub and the main
stack. -/
theorem fault_stub_mem (s h : M) (hop : Int) :
    ∀ x, s.sp ≤ x → (kentry_after_call (call_ret (fault_to_call s) h hop)).mem x = s.mem x := by
  intro x hx
  have hsp : (fault_to_call s).sp = s.sp - 8 := by
    simp only [fault_to_call, i_push_r4_lr, i_mrs_primask, i_tst_lr, i_mov_imm_eq, setr]
    split_ifs <;> rfl
  have hm : (fault_to_call s).mem = (i_push_r4_lr s).mem := by
    simp only [fault_to_call, i_push_r4_lr, i_mrs_primask, i_tst_lr, i_mov_imm_eq, setr]
    split_ifs <;> rfl
  simp only [kentry_after_call, i_pop_r4_lr, call_ret, call, setr]
  rw [hsp, hm]
  simp only [i_push_r4_lr]
  split_ifs <;> first | rfl | omega

/-- **The fault handlers' stub with `fault_core`.** -/
theorem fault_stub_world [KSpec ki kv okK] {k k' : K} {hwd hwd' : A} {tables : T}
    {s0 s1 : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} {kind : U8} {in_kernel : Bool}
    (s h : M) {g : Int} {ws : List (Nat × Stub.Frame)} {tp : Bool}
    (hok : okK k) (hai : AI kv k s0 0) (hw : World s0.levels s.mem (s.sp + g) ws tp)
    (he : hwEntry s.lr g tp)
    (hc : arch.cm4.entry.fault_core ki ai ti k hwd tables s0 kind in_kernel
      = ok ((hop, safe), k', hwd', s1))
    (h8 : hop.val ≠ 0 → hasBit s.lr 8 = true) :
    okK k' ∧ AI kv k' s1 hop.val ∧
    World s1.levels (kentry_after_call (call_ret (fault_to_call s) h hop.val)).mem (s.sp + g) ws tp ∧
    (hop.val ≠ 0 → World s1.levels (kentry_after_call (call_ret (fault_to_call s) h hop.val)).mem
      (kentry_after_call (call_ret (fault_to_call s) h hop.val)).sp ws tp) := by
  obtain ⟨hok', hai'⟩ := fault_core_keeps hok hai hc
  obtain ⟨_, t1, _⟩ := fault_stub s h hop.val
  have hg := he.1
  obtain ⟨w1, w2⟩ := kentry_world hw he (fault_core_frame hc) hai' t1
    (fun x hx => fault_stub_mem s h hop.val x (by omega)) h8
  exact ⟨hok', hai', w1, w2⟩

/-- `rsk_svc_entry`'s stub and the main stack: the service may write the
caller's exception frame, which on the main stack lies below the records. -/
theorem svc_stub_mem (s h : M) (hop : Int) (g : Int) (hg : hasBit s.lr 4 = false → 32 ≤ g)
    (hg0 : 0 ≤ g) :
    ∀ x, s.sp + g ≤ x →
      (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4) hop)).mem x
        = s.mem x := by
  intro x hx
  cases hb : hasBit s.lr 4
  · have := hg hb
    simp only [kentry_after_call, i_pop_r4_lr, call_svc, call, setr, svc_to_call, i_push_r4_lr,
      i_mov_r_lr, i_mrs_psp_ne, i_mrs_msp_eq, i_tst_lr, hb]
    simp
    split_ifs <;> first | rfl | omega
  · simp only [kentry_after_call, i_pop_r4_lr, call_svc, call, setr, svc_to_call, i_push_r4_lr,
      i_mov_r_lr, i_mrs_psp_ne, i_mrs_msp_eq, i_tst_lr, hb]
    simp
    split_ifs <;> first | rfl | omega

/-- **`rsk_svc_entry` with `svc_core`.** An SVC is taken from Thread mode. -/
theorem svc_stub_world [KSpecD ki kv okK] {k k' : K} {hwd hwd' : A} {tables : T}
    {s0 s1 : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool}
    {frame frame' : Array U32 8#usize} {number : U8} (s h : M) {g : Int}
    {ws : List (Nat × Stub.Frame)} {tp : Bool}
    (hok : okK k) (hai : AI kv k s0 0) (hw : World s0.levels s.mem (s.sp + g) ws tp)
    (he : hwEntry s.lr g tp) (h8 : hasBit s.lr 8 = true)
    (hc : arch.cm4.entry.svc_core ki ai ti k hwd tables s0 frame number
      = ok ((hop, safe), k', hwd', s1, frame')) :
    okK k' ∧ AI kv k' s1 hop.val ∧
    World s1.levels
      (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4)
        hop.val)).mem (s.sp + g) ws tp ∧
    (hop.val ≠ 0 → World s1.levels
      (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4)
        hop.val)).mem
      (kentry_after_call (call_svc (svc_to_call s) h ((svc_to_call s).r 0) (!hasBit s.lr 4)
        hop.val)).sp ws tp) := by
  obtain ⟨hok', hai'⟩ := svc_core_keeps hok hai hc
  obtain ⟨_, _, t1, _⟩ := svc_stub s h hop.val
  obtain ⟨w1, w2⟩ := kentry_world hw he (svc_core_frame hc) hai' t1
    (svc_stub_mem s h hop.val g he.2.2.2 he.1) (fun _ => h8)
  exact ⟨hok', hai', w1, w2⟩

/-! ## The dispatch stub -/

set_option maxHeartbeats 1000000 in
/-- **`rsk_irq_entry` with `irq_entry_core`.** Entered in state `s` with
the agreement, the Rust half called with the entry's LR as its EXC_RETURN
argument and answering `plan` without the safe state:

- on a dispatch the stub leaves the level's register record, built from
  the entry's registers (`entryFrame s g`), on top of the main stack, and
  the agreement holds with it;
- with no dispatch and no plan it returns as entered and the agreement
  holds as it did;
- with a plan it reaches `rsk_unwind` with the records at its stack
  pointer. -/
theorem irq_stub_world [KSpecD ki kv okK] {k k' : K} {hwd hwd' : A} {tables : T}
    {s0 s1 : arch.cm4.plan.ArchState} {ipsr psp control exc_return : U32}
    {plan : arch.cm4.entry.DispatchPlan} (s h : M) {g : Int} {ws : List (Nat × Stub.Frame)}
    {tp : Bool}
    (hok : okK k) (hai : AI kv k s0 0) (hw : World s0.levels s.mem (s.sp + g) ws tp)
    (he : hwEntry s.lr g tp) (hlr : s.lr = (exc_return.val : Int))
    (hc : arch.cm4.entry.irq_entry_core ki ai ti k hwd tables s0 ipsr psp control exc_return
      = ok ((plan, false), k', hwd', s1)) :
    okK k' ∧ AI kv k' s1 plan.hop.val ∧
    (plan.psp ≠ 0#u32 → ∃ level,
      World s1.levels
        (irq_dispatch (irq_called s h plan.psp.val plan.control.val plan.exc_return.val
          plan.hop.val)).mem
        (irq_dispatch (irq_called s h plan.psp.val plan.control.val plan.exc_return.val
          plan.hop.val)).sp ((level, entryFrame s g) :: ws) true) ∧
    (plan.psp = 0#u32 →
      World s1.levels
        (irq_restore (irq_called s h plan.psp.val plan.control.val plan.exc_return.val
          plan.hop.val)).mem (s.sp + g) ws tp ∧
      (plan.hop.val ≠ 0 → World s1.levels
        (irq_restore (irq_called s h plan.psp.val plan.control.val plan.exc_return.val
          plan.hop.val)).mem
        (irq_restore (irq_called s h plan.psp.val plan.control.val plan.exc_return.val
          plan.hop.val)).sp ws tp)) := by
  obtain ⟨hok', hai'⟩ := irq_entry_core_keeps hok hai hc
  obtain ⟨hir, hb8, hhop8⟩ := irq_entry_core_ir hok hai hc
  have hg := he.1
  refine ⟨hok', hai', fun hne => ?_, fun hz => ?_⟩
  · obtain ⟨level, l8, hsame, hex, hact, hlow⟩ := hir.2 hne
    obtain ⟨_, _, _, _, _, d6, d7, d8⟩ :=
      irq_dispatch_spec s h plan.psp.val plan.control.val plan.exc_return.val plan.hop.val g hg
    have h8 : hasBit s.lr 8 = true := by rw [hlr]; exact hasBit8 (hb8 hne)
    refine ⟨level, hw.push l8 hsame hex hact hlow d7 ?_ (fun x hx => d8 x (by omega)) ?_ ?_ ?_⟩
    · rw [d6]; simp only [entryFrame]; omega
    · simp only [entryFrame]; rw [hlr]
    · simp only [entryFrame]; rw [hlr]; simp
    · intro hne'
      have htp := hw.tp hne'
      have h4 := (he.2.1 h8).mpr htp
      refine ⟨he.2.2.1 h8 h4, ?_, ?_⟩
      · rw [← hlr]; exact h8
      · rw [← hlr]; exact h4
  · obtain ⟨r1, _, _, _, _, _, _, _, _, _, r11⟩ :=
      irq_restore_spec s h plan.psp.val plan.control.val plan.exc_return.val plan.hop.val
    have hfr : s1.levels = s0.levels := hir.1 hz
    have hw1 := (hw.mem_ext (fun x hx => r11 x (by omega))).levels_eq hfr
    refine ⟨hw1, fun hh => ?_⟩
    have h8 : hasBit s.lr 8 = true := by
      rw [hlr]
      exact hasBit8 (hhop8 rfl (fun e => hh (by rw [e]; rfl)))
    have hg0 := plan_g0 hai' hh hw1 he h8
    rw [r1]
    rw [hg0] at hw1
    simpa using hw1

/-! ## `rsk_unwind` -/

/-- **One round of `rsk_unwind` with `epilogue_core`.** Entered with the
agreement and the invariant in the form the plan `n` left, the Rust half
called with that plan: the round pops exactly the register record of the
level the core unwound (the topmost), R3 to R11 and LR come back from that
record, the EXC_RETURN value the core returns is the one the record holds
as its saved LR, and the agreement holds for the levels that remain. -/
theorem unwind_round_world [KSpec ki kv okK] {k k' : K} {hwd hwd' : A} {tables : T}
    {s0 s1 : arch.cm4.plan.ArchState} {n p c e nx : U32} {hop : Nat} {safe : Bool} (m h : M)
    {ws : List (Nat × Stub.Frame)} {tp : Bool}
    (hok : okK k) (hai : AI kv k s0 hop) (hhop : hop ≠ 0)
    (hn : (UScalar.cast .U16 (n &&& 511#u32)).val = hop)
    (hw : World s0.levels m.mem m.sp ws tp)
    (hc : arch.cm4.entry.epilogue_core ki ai ti k hwd tables s0 n
      = ok ((((p, c, e, nx), safe), k', hwd', s1))) :
    okK k' ∧ AI kv k' s1 nx.val ∧
    ∃ level f rest, ws = (level, f) :: rest ∧
      (unwind_round m h p.val c.val e.val nx.val).sp = m.sp + frameSize f ∧
      (∀ i, 3 ≤ i → i ≤ 11 → (unwind_round m h p.val c.val e.val nx.val).r i = f.regs (i - 3)) ∧
      (unwind_round m h p.val c.val e.val nx.val).lr = f.regs 9 ∧
      (unwind_round m h p.val c.val e.val nx.val).psp = p.val ∧
      (unwind_round m h p.val c.val e.val nx.val).control = c.val ∧
      (unwind_round m h p.val c.val e.val nx.val).r 2 = e.val ∧
      (unwind_round m h p.val c.val e.val nx.val).z = decide ((nx.val : Int) = 0) ∧
      (unwind_round m h p.val c.val e.val nx.val).primask = m.primask ∧
      (unwind_round m h p.val c.val e.val nx.val).faultmask = m.faultmask ∧
      (∀ x, m.sp ≤ x → (unwind_round m h p.val c.val e.val nx.val).mem x = m.mem x) ∧
      (rest ≠ [] → f.gap = 0) ∧
      f.regs 9 = (e.val : Int) ∧
      World s1.levels (unwind_round m h p.val c.val e.val nx.val).mem
        ((unwind_round m h p.val c.val e.val nx.val).sp + f.gap) rest
        (hasBit (e.val : Int) 8 && hasBit (e.val : Int) 4) := by
  obtain ⟨hok', hai'⟩ := epilogue_core_keeps_all hok hai hhop hn hc
  obtain ⟨level, htl, hsame, hoff⟩ := epilogue_core_pop hok hai hhop hn hc
  obtain ⟨level', rc, htl', hrc, rfl, rfl, rfl⟩ := epilogue_returns_record hc
  have hll : level' = level := ok_inj (htl'.symm.trans htl)
  subst hll
  obtain ⟨_, hrce⟩ := index_ok hrc
  obtain ⟨⟨l0, ha0, hl0⟩, _⟩ := hai.2.1 hhop
  have htop := top_active_isTop htl ha0 (by omega)
  obtain ⟨f, rest, rfl⟩ := hw.top htop
  obtain ⟨hfo, hup, _⟩ := hw.ok
  have hexc : excOf s0.levels level'.val = (rc.exc_return.val : Int) := by
    simp only [excOf]; rw [hrce]
  have hft : frameTop m.mem m.sp f := hw.frames.1
  have hz : f.hasFp = !hasBit (rc.exc_return.val : Int) 16 := by rw [← hexc]; exact hfo.1
  obtain ⟨u1, u2, u3, u4, u5, u6, u7, u8, u9, u10, u11, u12⟩ :=
    unwind_round_spec m h rc.psp.val rc.control.val rc.exc_return.val nx.val f hft hz
  refine ⟨hok', hai', level'.val, f, rest, rfl, u1, u2, u3, u5, u6, u7, u9, u10, u11, u12,
    fun hne => (hup hne).1, by rw [← hexc]; exact hfo.2, ?_⟩
  have hpop := hw.pop (m' := (unwind_round m h rc.psp.val rc.control.val rc.exc_return.val
    nx.val).mem) (tp' := (hasBit (rc.exc_return.val : Int) 8 && hasBit (rc.exc_return.val : Int) 4))
    hsame hoff u12 (fun hne => by
      obtain ⟨_, t8, t4⟩ := hup hne
      rw [hexc] at t8 t4
      simp [t8, t4])
  rw [u1]
  have e : m.sp + frameSize f + f.gap = m.sp + frameSize f + f.gap := rfl
  exact hpop

end P18
