import Hw.Model
open Aeneas Aeneas.Std Result

/-! # The register drivers return (2026-11-20)

`Hw/Model.lean` proves what each driver of `arch/cm4/drivers.rs` does to a
register file when it returns. Here: each returns, for every argument, over
any register file whose reads and writes return (`RegsT`). This is the part
of "the hardware operations return" (`HwT`, `Sched/CoreTotal.lean`) that is
Kernel code. The functions between `KernelArch::apply` and these drivers
(`unsafe_module/hw.rs`, the volatile accesses) are not translated: they
dereference raw pointers. -/

namespace Hw
open rsk_kernel rsk_kernel.arch.cm4 rsk_kernel.arch.cm4.drivers

variable {R E : Type} {ri : Regs R} {ei : Extend E}

/-- The register file's reads and writes return. -/
class RegsT (ri : Regs R) : Prop where
  read : ∀ (r : R) (a : U32), ri.read r a ⦃ _ => True ⦄
  write : ∀ (r : R) (a v : U32), ri.write r a v ⦃ _ => True ⦄
  write_u8 : ∀ (r : R) (a : U32) (v : U8), ri.write_u8 r a v ⦃ _ => True ⦄

@[step] theorem regs_read [RegsT ri] (r : R) (a : U32) : ri.read r a ⦃ _ => True ⦄ :=
  RegsT.read r a
@[step] theorem regs_write [RegsT ri] (r : R) (a v : U32) : ri.write r a v ⦃ _ => True ⦄ :=
  RegsT.write r a v
@[step] theorem regs_write_u8 [RegsT ri] (r : R) (a : U32) (v : U8) :
    ri.write_u8 r a v ⦃ _ => True ⦄ := RegsT.write_u8 r a v

/-- The extension of the timer's 32 bits to the 64-bit time base returns. -/
class ExtendT (ei : Extend E) : Prop where
  extend : ∀ (e : E) (low : U32), ei.extend e low ⦃ _ => True ⦄

@[step] theorem ext_extend [ExtendT ei] (e : E) (low : U32) : ei.extend e low ⦃ _ => True ⦄ :=
  ExtendT.extend e low

/-- Steps through a body, splitting its conditionals. -/
macro "htac" : tactic => `(tactic| (step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*) <;> try (split <;> step* <;> (try (casesm* _ × _)) <;> (try simp only []) <;> (try step*))))))))

macro "hcl" : tactic => `(tactic| all_goals first
  | (scalar_tac; done)
  | (simp_all; done)
  | (simp_all; scalar_tac))

@[step] theorem timer0_vector_t : timer0.TIMER0_VECTOR ⦃ _ => True ⦄ := by
  unfold timer0.TIMER0_VECTOR
  step*

/-- `irq_of` answers a line of the NVIC. -/
@[step] theorem irq_of_t (n : U16) :
    irq_of n ⦃ r => match r with | none => True | some i => i.val < 496 ⦄ := by
  unfold irq_of
  simp only [addr.NVIC_LINES]
  htac
  all_goals first
    | (scalar_tac; done)
    | (simp_all; done)
    | (simp_all; scalar_tac)

@[step] theorem nvic_enable_t [RegsT ri] (r : R) (n : U16) :
    nvic_enable ri r n ⦃ _ => True ⦄ := by
  unfold nvic_enable
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem nvic_disable_t [RegsT ri] (r : R) (n : U16) :
    nvic_disable ri r n ⦃ _ => True ⦄ := by
  unfold nvic_disable
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem nvic_set_pending_t [RegsT ri] (r : R) (n : U16) :
    nvic_set_pending ri r n ⦃ _ => True ⦄ := by
  unfold nvic_set_pending
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem nvic_clear_pending_t [RegsT ri] (r : R) (n : U16) :
    nvic_clear_pending ri r n ⦃ _ => True ⦄ := by
  unfold nvic_clear_pending
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem nvic_set_priority_t [RegsT ri] (r : R) (n : U16) (encoded : U8) :
    nvic_set_priority ri r n encoded ⦃ _ => True ⦄ := by
  unfold nvic_set_priority
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem set_prigroup_all_preempt_t [RegsT ri] (r : R) :
    set_prigroup_all_preempt ri r ⦃ _ => True ⦄ := by
  unfold set_prigroup_all_preempt
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem set_nonbasethrdena_t [RegsT ri] (r : R) (on : Bool) :
    set_nonbasethrdena ri r on ⦃ _ => True ⦄ := by
  unfold set_nonbasethrdena
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem set_disdefwbuf_t [RegsT ri] (r : R) (on : Bool) :
    set_disdefwbuf ri r on ⦃ _ => True ⦄ := by
  unfold set_disdefwbuf
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem set_kernel_exception_priorities_t [RegsT ri] (r : R) :
    set_kernel_exception_priorities ri r ⦃ _ => True ⦄ := by
  unfold set_kernel_exception_priorities
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem fpu_configure_eager_t [RegsT ri] (r : R) (flush_to_zero : Bool) :
    fpu_configure_eager ri r flush_to_zero ⦃ _ => True ⦄ := by
  unfold fpu_configure_eager
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem mpu_enable_background_t [RegsT ri] (r : R) :
    mpu_enable_background ri r ⦃ _ => True ⦄ := by
  unfold mpu_enable_background
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem dwt_enable_t [RegsT ri] (r : R) :
    dwt_enable ri r ⦃ _ => True ⦄ := by
  unfold dwt_enable
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem timer0_now_t [RegsT ri] [ExtendT ei] (r : R) (e : E) :
    timer0.now ri ei r e ⦃ _ => True ⦄ := by
  unfold timer0.now
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem timer0_arm_t [RegsT ri] [ExtendT ei] (r : R) (e : E) («at» : U64) :
    timer0.arm ri ei r e «at» ⦃ _ => True ⦄ := by
  unfold timer0.arm
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem timer0_ack_t [RegsT ri] (r : R) :
    timer0.ack ri r ⦃ _ => True ⦄ := by
  unfold timer0.ack
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem timer0_cancel_t [RegsT ri] (r : R) :
    timer0.cancel ri r ⦃ _ => True ⦄ := by
  unfold timer0.cancel
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

@[step] theorem timer0_rearm_t [RegsT ri] [ExtendT ei] (r : R) (e : E) («at» : U64) :
    timer0.rearm ri ei r e «at» ⦃ _ => True ⦄ := by
  unfold timer0.rearm
  try simp only [addr.ACTLR, addr.NVIC_ISER, addr.NVIC_ICER, addr.NVIC_ISPR, addr.NVIC_ICPR, addr.NVIC_IPR, addr.AIRCR, addr.CCR, addr.SHPR1, addr.SHPR2, addr.SHPR3, addr.SHCSR, addr.CPACR, addr.MPU_CTRL, addr.DEMCR, addr.FPCCR, addr.FPDSCR, addr.DWT_CTRL, addr.DWT_CYCCNT, addr.NRF_TIMER0, addr.NVIC_LINES, timer0.TASKS_CAPTURE1, timer0.EVENTS_COMPARE0, timer0.INTENSET, timer0.INTENCLR, timer0.CC0, timer0.CC1]
  htac
  hcl

end Hw
