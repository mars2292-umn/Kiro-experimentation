import Sched.World
import Sched.ArchFaultK
open Aeneas Aeneas.Std Result

/-! # The rest of the stub theorems (2026-11-20)

What `Sched/World.lean` left open of `stub_model.rs`:

- `mm_stub_world`: `rsk_memmanage_entry` with `fault_core`;
- `nested_irq_world`, `nested_fault_world`: the entry-instruction window
  (5.8a). A handler preempted by an interrupt vector or a fault finds its
  registers, its stack and the agreement as it left them, because for an
  entry from Handler mode the Rust half plans neither a dispatch nor an
  unwind;
- `unwind_loop_world`: `rsk_unwind` as one statement, over any number of
  rounds, each calling `epilogue_core` with the plan the previous round
  returned. -/

namespace P18
open rsk_kernel Stub

variable {K A T : Type} {ki : arch.bound.KernelOps K} {ai : arch.cm4.entry.KernelArch A}
  {ti : arch.cm4.entry.Tables T} {kv : K → Arch.KV} {okK : K → Prop}

/-! ## `rsk_memmanage_entry` -/

theorem mm_stub_mem (s h : M) (hop : Int) :
    ∀ x, s.sp ≤ x → (kentry_after_call (call_ret (mm_to_call s) h hop)).mem x = s.mem x := by
  intro x hx
  have hsp : (mm_to_call s).sp = s.sp - 8 := by
    cases hb : hasBit s.lr 8 <;> cases hb4 : hasBit s.lr 4 <;> cases hp : s.primask <;>
      simp [mm_to_call, i_push_r4_lr, i_mrs_primask, i_tst_lr, i_mov_imm_eq, i_mrs_psp_ne,
        i_add_r_sp_eq, i_mov_r_lr, setr, hb, hb4, hp]
  have hm : (mm_to_call s).mem = (i_push_r4_lr s).mem := by
    cases hb : hasBit s.lr 8 <;> cases hb4 : hasBit s.lr 4 <;> cases hp : s.primask <;>
      simp [mm_to_call, i_push_r4_lr, i_mrs_primask, i_tst_lr, i_mov_imm_eq, i_mrs_psp_ne,
        i_add_r_sp_eq, i_mov_r_lr, setr, hb, hb4, hp]
  simp only [kentry_after_call, i_pop_r4_lr, call_ret, call, setr]
  rw [hsp, hm]
  simp only [i_push_r4_lr]
  split_ifs <;> first | rfl | omega

/-- **`rsk_memmanage_entry` with `fault_core`.** -/
theorem mm_stub_world [KSpec ki kv okK] {k k' : K} {hwd hwd' : A} {tables : T}
    {s0 s1 : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} {kind : U8} {in_kernel : Bool}
    (s h : M) {g : Int} {ws : List (Nat × Stub.Frame)} {tp : Bool}
    (hok : okK k) (hai : AI kv k s0 0) (hw : World s0.levels s.mem (s.sp + g) ws tp)
    (he : hwEntry s.lr g tp)
    (hc : arch.cm4.entry.fault_core ki ai ti k hwd tables s0 kind in_kernel
      = ok ((hop, safe), k', hwd', s1))
    (h8 : hop.val ≠ 0 → hasBit s.lr 8 = true) :
    okK k' ∧ AI kv k' s1 hop.val ∧
    World s1.levels (kentry_after_call (call_ret (mm_to_call s) h hop.val)).mem (s.sp + g) ws tp ∧
    (hop.val ≠ 0 → World s1.levels (kentry_after_call (call_ret (mm_to_call s) h hop.val)).mem
      (kentry_after_call (call_ret (mm_to_call s) h hop.val)).sp ws tp) := by
  obtain ⟨hok', hai'⟩ := fault_core_keeps hok hai hc
  obtain ⟨_, _, _, t1, _⟩ := memmanage_stub s h hop.val
  have hg := he.1
  obtain ⟨w1, w2⟩ := kentry_world hw he (fault_core_frame hc) hai' t1
    (fun x hx => mm_stub_mem s h hop.val x (by omega)) h8
  exact ⟨hok', hai', w1, w2⟩

/-! ## The entry-instruction window -/

theorem handler_lr_bit8 : (4294967281#u32 &&& 8#u32) = 0#u32 := by decide

/-- **An interrupt vector taken while a handler executes.** The handler,
in state `s` with the agreement for its own entry (`g0`), is preempted;
the preempting stub's Rust half is `irq_entry_core`, called with the
Handler-mode EXC_RETURN value. It plans neither a dispatch nor an unwind;
the stub returns as entered; and the exception return gives the preempted
handler back every register, its stack pointer and the memory at and above
it, with the agreement for the Kernel state the preempting entry left. -/
theorem nested_irq_world [KSpecD ki kv okK] {k k' : K} {hwd hwd' : A} {tables : T}
    {s0 s1 : arch.cm4.plan.ArchState} {ipsr psp control : U32}
    {plan : arch.cm4.entry.DispatchPlan} (s hx h : M) (n g : Int) {g0 : Int}
    {ws : List (Nat × Stub.Frame)} {tp : Bool} (hg : 32 ≤ g) (hg0 : 0 ≤ g0)
    (hok : okK k) (hai : AI kv k s0 0) (hw : World s0.levels s.mem (s.sp + g0) ws tp)
    (hc : arch.cm4.entry.irq_entry_core ki ai ti k hwd tables s0 ipsr psp control 4294967281#u32
      = ok ((plan, false), k', hwd', s1)) :
    plan.psp = 0#u32 ∧ plan.hop = 0#u32 ∧ okK k' ∧ AI kv k' s1 0 ∧
    (∀ i, i ≤ 12 → (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 plan.control.val plan.exc_return.val 0))) g s.ipsr).r i
        = s.r i) ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 plan.control.val plan.exc_return.val 0))) g s.ipsr).sp
        = s.sp ∧
    (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 plan.control.val plan.exc_return.val 0))) g s.ipsr).lr
        = s.lr ∧
    World s1.levels (exc_return_handler (irq_return (irq_restore (irq_called
      (exc_enter_handler s n g hx) h 0 plan.control.val plan.exc_return.val 0))) g s.ipsr).mem
      (s.sp + g0) ws tp := by
  obtain ⟨hok', hai'⟩ := irq_entry_core_keeps hok hai hc
  obtain ⟨hir, hb8, hhop8⟩ := irq_entry_core_ir hok hai hc
  have hpsp : plan.psp = 0#u32 := by
    by_contra hne
    exact hb8 hne handler_lr_bit8
  have hhop : plan.hop = 0#u32 := by
    by_contra hne
    exact hhop8 rfl hne handler_lr_bit8
  obtain ⟨r1, r2, r3, _, _, _, _, _, _, r10⟩ :=
    nested_irq_resumed s hx h n g plan.control.val plan.exc_return.val hg
  have hv : plan.hop.val = 0 := by rw [hhop]; rfl
  rw [hv] at hai'
  refine ⟨hpsp, hhop, hok', hai', r1, r2, r3, ?_⟩
  exact (hw.mem_ext (fun x hx' => r10 x (by omega))).levels_eq (hir.1 hpsp)

/-- **A fault taken while a handler executes.** The fault stub passes it to
`fault_core` as the Kernel's, which plans no unwind; an entry that returns
gives the interrupted handler back its stack and the agreement. -/
theorem nested_fault_world [KSpecD ki kv okK] {k k' : K} {hwd hwd' : A} {tables : T}
    {s0 s1 : arch.cm4.plan.ArchState} {hop : U32} {safe : Bool} {kind : U8}
    (s hx h : M) (n g : Int) {g0 : Int} {ws : List (Nat × Stub.Frame)} {tp : Bool}
    (hg : 32 ≤ g) (hg0 : 0 ≤ g0)
    (hok : okK k) (hai : AI kv k s0 0) (hw : World s0.levels s.mem (s.sp + g0) ws tp)
    (hc : arch.cm4.entry.fault_core ki ai ti k hwd tables s0 kind true
      = ok ((hop, safe), k', hwd', s1)) :
    (fault_to_call (exc_enter_handler s n g hx)).r 0 ≠ 0 ∧ hop = 0#u32 ∧ okK k' ∧ AI kv k' s1 0 ∧
    (kentry_after_call (call_ret (fault_to_call (exc_enter_handler s n g hx)) h 0)).sp
      = s.sp - g ∧
    World s1.levels
      (kentry_after_call (call_ret (fault_to_call (exc_enter_handler s n g hx)) h 0)).mem
      (s.sp + g0) ws tp := by
  have hhop := fault_core_kernel_hop (kv := kv) hok hc
  obtain ⟨hok', hai'⟩ := fault_core_keeps hok hai hc
  have hv : hop.val = 0 := by rw [hhop]; rfl
  rw [hv] at hai'
  obtain ⟨f0, f1, _⟩ := fault_stub (exc_enter_handler s n g hx) h 0
  have hk : (fault_to_call (exc_enter_handler s n g hx)).r 0 ≠ 0 :=
    f0.mpr (Or.inr handler_exc_return.1)
  have mab : ∀ x, s.sp ≤ x → (exc_enter_handler s n g hx).mem x = s.mem x := by
    intro x hx'
    simp only [exc_enter_handler]
    split_ifs <;> first | rfl | omega
  have hmem : ∀ x, s.sp + g0 ≤ x →
      (kentry_after_call (call_ret (fault_to_call (exc_enter_handler s n g hx)) h 0)).mem x
        = s.mem x := by
    intro x hx'
    rw [fault_stub_mem (exc_enter_handler s n g hx) h 0 x (by show s.sp - g ≤ x; omega)]
    exact mab x (by omega)
  exact ⟨hk, hhop, hok', hai', f1, (hw.mem_ext hmem).levels_eq (fault_core_frame hc).1⟩

/-! ## `rsk_unwind`, the whole loop -/

/-- The rounds of an unwind on the Rust side: `epilogue_core` called with
the plan `n`, then with the plan each round returned, until one returns no
further plan; none reaches the safe state. Each round is listed with the
state the machine's call returned (`h`) and its answer. -/
inductive UnwRun (ki : arch.bound.KernelOps K) (ai : arch.cm4.entry.KernelArch A)
    (ti : arch.cm4.entry.Tables T) (tables : T) :
    K → A → arch.cm4.plan.ArchState → U32 → List (M × U32 × U32 × U32 × U32) → K →
      arch.cm4.plan.ArchState → Prop
  | last {k k' : K} {hw hw' : A} {s s' : arch.cm4.plan.ArchState} {n p c e : U32} (h : M)
      (hc : arch.cm4.entry.epilogue_core ki ai ti k hw tables s n
        = ok ((((p, c, e, 0#u32), false), k', hw', s'))) :
      UnwRun ki ai ti tables k hw s n [(h, p, c, e, 0#u32)] k' s'
  | more {k k' k'' : K} {hw hw' : A} {s s' s'' : arch.cm4.plan.ArchState} {n p c e nx : U32}
      (h : M) {rest : List (M × U32 × U32 × U32 × U32)}
      (hc : arch.cm4.entry.epilogue_core ki ai ti k hw tables s n
        = ok ((((p, c, e, nx), false), k', hw', s')))
      (hnx : nx ≠ 0#u32) (hr : UnwRun ki ai ti tables k' hw' s' nx rest k'' s'') :
      UnwRun ki ai ti tables k hw s n ((h, p, c, e, nx) :: rest) k'' s''

/-- The machine's rounds: each answer with the register record it pops. -/
def mkRounds : List Stub.Frame → List (M × U32 × U32 × U32 × U32) →
    List (Stub.Frame × M × Int × Int × Int × Int)
  | f :: fs, (h, p, c, e, nx) :: rs => (f, h, p.val, c.val, e.val, nx.val) :: mkRounds fs rs
  | _, _ => []

theorem u32_ne_val {x : U32} (h : x ≠ 0#u32) : x.val ≠ 0 := fun e => h (by scalar_tac)

/-- **`rsk_unwind` with `epilogue_core`, over any number of rounds.**
Entered at its loop's head with the agreement and the invariant in the
form the plan `n` left: the loop pops exactly the register records of the
levels the rounds unwound (`pop`, as many as rounds), leaves by an
exception return (PRIMASK clear, FAULTMASK set) with R3 to R11 and LR of
the last record popped, the invariant holds with nothing planned, and the
agreement holds for the levels that remain. -/
theorem unwind_loop_world [KSpec ki kv okK] {tables : T} {k k' : K} {hwd : A}
    {s0 s' : arch.cm4.plan.ArchState} {n : U32} {rs : List (M × U32 × U32 × U32 × U32)}
    (hr : UnwRun ki ai ti tables k hwd s0 n rs k' s') :
    ∀ (m : M) (ws : List (Nat × Stub.Frame)) (tp : Bool), okK k → AI kv k s0 n.val →
      n.val ≠ 0 → m.primask = true → World s0.levels m.mem m.sp ws tp →
      okK k' ∧ AI kv k' s' 0 ∧
      ∃ (pop rest : List (Nat × Stub.Frame)) (lf : Stub.Frame) (tp' : Bool),
        ws = pop ++ rest ∧ pop.length = rs.length ∧ (pop.map Prod.snd).getLast? = some lf ∧
        (unwind_loop m (mkRounds (pop.map Prod.snd) rs)).primask = false ∧
        (unwind_loop m (mkRounds (pop.map Prod.snd) rs)).faultmask = true ∧
        (∀ i, 3 ≤ i → i ≤ 11 →
          (unwind_loop m (mkRounds (pop.map Prod.snd) rs)).r i = lf.regs (i - 3)) ∧
        (unwind_loop m (mkRounds (pop.map Prod.snd) rs)).lr = lf.regs 9 ∧
        World s'.levels (unwind_loop m (mkRounds (pop.map Prod.snd) rs)).mem
          ((unwind_loop m (mkRounds (pop.map Prod.snd) rs)).sp + lf.gap) rest tp' := by
  induction hr with
  | @last k k' hw hw' s s' n p c e h hc =>
    intro m ws tp hok hai hn hpm hw
    obtain ⟨hok', hai', level, f, rest, rfl, u1, u2, u3, _, _, _, uz, upm, ufm, umem, _, _, hw'⟩ :=
      unwind_round_world m h hok hai hn (plan_mask hai hn) hw hc
    refine ⟨hok', hai', [(level, f)], rest, f, (hasBit (e.val : Int) 8 && hasBit (e.val : Int) 4),
      rfl, rfl, rfl, ?_⟩
    have hz : (unwind_round m h p.val c.val e.val (0#u32).val).z = true := by rw [uz]; rfl
    simp only [List.map_cons, List.map_nil, mkRounds, unwind_loop, hz, if_true]
    refine ⟨rfl, rfl, fun i h3 h11 => u2 i h3 h11, u3, hw'⟩
  | @more k k' k'' hw hw' s s' s'' n p c e nx h rest hc hnx hr ih =>
    intro m ws tp hok hai hn hpm hw
    obtain ⟨hok', hai', level, f, rest1, rfl, u1, u2, u3, _, _, _, uz, upm, ufm, umem, hgap, _,
      hw'⟩ := unwind_round_world m h hok hai hn (plan_mask hai hn) hw hc
    have hnv := u32_ne_val hnx
    obtain ⟨⟨l, ha, _⟩, _⟩ := hai'.2.1 hnv
    have hne1 : rest1 ≠ [] := hw'.nonempty ha
    rw [hgap hne1] at hw'
    -- the machine goes round again from the same stack
    obtain ⟨hok'', hai'', pop, rest0, lf, tp', rfl, hlen, hlast, c1, c2, c3, c4, c5⟩ :=
      ih (unwind_again (unwind_round m h p.val c.val e.val nx.val)) rest1 _ hok' hai' hnv (by
        show (unwind_round m h p.val c.val e.val nx.val).primask = true
        rw [upm]; exact hpm) (by simpa [unwind_again, i_mov, setr] using hw')
    have hz : (unwind_round m h p.val c.val e.val nx.val).z = false := by
      rw [uz]
      simp
      exact_mod_cast hnv
    refine ⟨hok'', hai'', (level, f) :: pop, rest0, lf, tp', rfl, by simp [hlen], ?_, ?_⟩
    · cases pop with
      | nil => simp at hlen; cases hr <;> simp at hlen
      | cons x xs => simpa using hlast
    · simp only [List.map_cons, mkRounds, unwind_loop, hz]
      exact ⟨c1, c2, c3, c4, c5⟩

end P18
