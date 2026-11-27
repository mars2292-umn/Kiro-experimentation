/-
  The ELF reader on the code as compiled (2026-11-26): the statements
  Verus proves of `rsk_elf::reader_v`, proven of the functions as Aeneas
  extracts them (`Tools/Elf.lean`). Each theorem is total: the function
  returns for the inputs named, and its result satisfies the
  postcondition. Handwritten.

  - `u16_at_spec`, `u32_at_spec`: a little-endian word is read exactly
    when the file has its bytes; no read is out of range.
  - `str_end_spec`: the end of a zero-terminated string.
  - `section_range_spec`: a section's extent in the file.
  - `sym_entry_spec`, `sym_entries_spec`, `symbols_spec`: the symbols of
    every symbol table: each an entry of a table, in table order, none
    missing.
  - `name_is_spec`, `find_spec`, `lookup_spec`: the lookup by name
    returns the first entry, in table order, whose name bytes in the
    string table are the name asked for (an object when asked), with that
    entry's fields; and `none` only when no entry of any table is such.
  - `section_at_spec`, `copy_range_spec`, `zeros_spec`, `read_at_spec`:
    the bytes at a virtual address are the file's bytes at the offset of
    the first allocated section holding the address plus the address's
    offset within it (zeros for a section without file content).
  - `rd_segments_spec`, `rd_sections_spec`, `rd_names_spec`,
    `no_names_spec`, `parse_spec`: the parse returns for every byte
    string; its header is the words at their offsets, its tables have the
    lengths the header states, and every name extent lies in the file.
    That each program and section header is the words at its offset
    (`parsed_ok`'s `segs_ok` and `secs_ok`) is proven in Verus only.

  The specification (`le16`, `le32`, `IsStr`, `SecRange`, `EntryOk`,
  `SymOk`, `ValidEntry`, `SymsOk`, `NameIs`, `Wanted`, `Before`,
  `SecHolds`, `BytesAt`, `HeaderOk`) restates the Verus definitions of
  `reader_v.rs` over Lean lists of bytes; that the two texts say the same
  is by reading. Two differ in form: `SecRange` is the relation of
  `sec_range`'s graph, and `NameIs` states the name's bytes one by one
  where Verus compares a subrange.

  One hypothesis the Verus statements do not have: `symbols_spec` and
  `lookup_spec` assume that the number of sections times the length of
  the file is below `Usize.max`. Aeneas models `Vec::push` as failing on a
  vector of `Usize.max` elements; the hypothesis bounds the number of
  symbols a file can yield (a table has at most one entry per byte of the
  file). It holds of every file of less than 2^48 bytes on a 64-bit host.
-/
import Aeneas
import Tools.Elf
open Aeneas Aeneas.Std Result

namespace rsk_elf

/-- Closes a branch in which the reader refuses the file. -/
macro "elf_done" : tactic =>
  `(tactic| first | (simp only [WP.spec_ok]; done) | (simp only [WP.spec_ok]; trivial) | trivial)

deriving instance Inhabited for reader_v.SecHdr
deriving instance Inhabited for reader_v.RawSym

/-! ## Casts and indexing -/

theorem usize_lt_u64 (x : Usize) : x.val < 2 ^ 64 := by
  have hb : x.val < 2 ^ System.Platform.numBits := x.hBounds
  rcases System.Platform.numBits_eq with h | h
  · rw [h] at hb; omega
  · rw [h] at hb; omega

theorem cast_usize_u64 (x : Usize) : (UScalar.cast .U64 x).val = x.val := by
  rw [UScalar.cast_val_eq]
  exact Nat.mod_eq_of_lt (usize_lt_u64 x)

theorem cast_u64_usize (x : U64) (y : Usize) (h : x.val ≤ y.val) :
    (UScalar.cast .Usize x).val = x.val := by
  rw [UScalar.cast_val_eq]
  exact Nat.mod_eq_of_lt (Nat.lt_of_le_of_lt h y.hBounds)

theorem slice_len_val (d : Slice U8) : d.len.val = d.val.length := by
  first | rfl | scalar_tac | simp

theorem slice_lt_u64 (d : Slice U8) : d.val.length < 2 ^ 64 := by
  have h := usize_lt_u64 d.len
  rw [slice_len_val] at h
  exact h

/-- An element read at an index, as the element at an equal index. -/
theorem bang_of_eq {α} [Inhabited α] {l : List α} {i j : Nat} {x : α} (h : i < l.length)
    (hx : x = l[i]) (hij : i = j) : l[j]! = x := by
  subst hij
  rw [hx]
  exact getElem!_pos l i h

/-! ## The statements -/

/-- A `u16`, least significant byte first, at `o` (`reader_v::le16`). -/
def le16 (d : List U8) (o : Nat) : BitVec 16 :=
  d[o]!.bv.setWidth 16 ||| (d[o + 1]!.bv.setWidth 16 <<< (8 : Nat))

/-- A `u32`, least significant byte first, at `o` (`reader_v::le32`). -/
def le32 (d : List U8) (o : Nat) : BitVec 32 :=
  d[o]!.bv.setWidth 32 ||| (d[o + 1]!.bv.setWidth 32 <<< (8 : Nat)) |||
    (d[o + 2]!.bv.setWidth 32 <<< (16 : Nat)) ||| (d[o + 3]!.bv.setWidth 32 <<< (24 : Nat))

/-- `e` is where the zero-terminated string that starts at `frm` ends, in
a table that ends at `lim` (`reader_v::is_str`). -/
def IsStr (d : List U8) (frm lim e : Nat) : Prop :=
  frm ≤ e ∧ e ≤ lim ∧ lim ≤ d.length ∧ (∀ i, frm ≤ i → i < e → d[i]! ≠ 0#u8) ∧
  (e < lim → d[e]! = 0#u8)

theorem IsStr.unique {d : List U8} {frm lim e1 e2 : Nat} (h1 : IsStr d frm lim e1)
    (h2 : IsStr d frm lim e2) : e1 = e2 := by
  obtain ⟨a1, a2, a3, a4, a5⟩ := h1
  obtain ⟨b1, b2, b3, b4, b5⟩ := h2
  by_contra hne
  rcases Nat.lt_or_gt_of_ne hne with h | h
  · exact b4 e1 a1 h (a5 (by omega))
  · exact a4 e2 b1 h (b5 (by omega))

/-- A section's extent `[a, b)` in a file of `len` bytes: empty for a
section without file content (`reader_v::sec_range`, as a relation). -/
def SecRange (len : Nat) (s : reader_v.SecHdr) (a b : Nat) : Prop :=
  (s.kind = reader_v.SHT_NOBITS ∧ a = 0 ∧ b = 0) ∨
  (s.kind ≠ reader_v.SHT_NOBITS ∧ s.offset.val + s.size.val ≤ len ∧ a = s.offset.val ∧
    b = s.offset.val + s.size.val)

theorem SecRange.le {len : Nat} {s : reader_v.SecHdr} {a b : Nat} (h : SecRange len s a b) :
    a ≤ b ∧ b ≤ len := by
  rcases h with ⟨_, h1, h2⟩ | ⟨_, h1, h2, h3⟩ <;> omega

theorem SecRange.unique {len : Nat} {s : reader_v.SecHdr} {a b a' b' : Nat}
    (h1 : SecRange len s a b) (h2 : SecRange len s a' b') : a = a' ∧ b = b' := by
  rcases h1 with ⟨k1, x1, y1⟩ | ⟨k1, _, x1, y1⟩ <;> rcases h2 with ⟨k2, x2, y2⟩ | ⟨k2, _, x2, y2⟩
  · omega
  · exact absurd k1 k2
  · exact absurd k2 k1
  · omega

/-- The size of a symbol table's entries (`reader_v::ent_size`). -/
def entSize (s : reader_v.SecHdr) : Nat := if s.entsize = 0#u32 then 16 else s.entsize.val

/-- `s` holds the fields of the entry of `e` bytes at `b`, with its name
in the string table `[ss, se)` (`reader_v::entry_ok`). -/
def EntryOk (d : List U8) (ss se b e : Nat) (s : reader_v.RawSym) : Prop :=
  16 ≤ e ∧ s.value.bv = le32 d (b + 4) ∧ s.size.bv = le32 d (b + 8) ∧ s.info = d[b + 12]! ∧
  s.shndx.bv = le16 d (b + 14) ∧ s.name_start.val = ss + (le32 d b).toNat ∧
  IsStr d s.name_start.val se s.name_end.val

/-- Section `t` is a symbol table with a string table, both in the file,
and has an entry of index `k` (`reader_v::valid_entry`). -/
def ValidEntry (d : List U8) (secs : List reader_v.SecHdr) (t k : Nat) : Prop :=
  t < secs.length ∧ secs[t]!.kind = reader_v.SHT_SYMTAB ∧ secs[t]!.link.val < secs.length ∧
  ∃ ss se ts te, SecRange d.length secs[secs[t]!.link.val]! ss se ∧
    SecRange d.length secs[t]! ts te ∧ ts + k * entSize secs[t]! + entSize secs[t]! ≤ te

/-- `s` is the entry `s.index` of the symbol table `s.table`
(`reader_v::sym_ok`). -/
def SymOk (d : List U8) (secs : List reader_v.SecHdr) (s : reader_v.RawSym) : Prop :=
  s.table.val < secs.length ∧ secs[s.table.val]!.kind = reader_v.SHT_SYMTAB ∧
  secs[s.table.val]!.link.val < secs.length ∧
  ∃ ss se ts te, SecRange d.length secs[secs[s.table.val]!.link.val]! ss se ∧
    SecRange d.length secs[s.table.val]! ts te ∧
    ts + s.index.val * entSize secs[s.table.val]! + entSize secs[s.table.val]! ≤ te ∧
    EntryOk d ss se (ts + s.index.val * entSize secs[s.table.val]!) (entSize secs[s.table.val]!) s

/-- Table order (`reader_v::before`). -/
def Before (a b : reader_v.RawSym) : Prop :=
  a.table.val < b.table.val ∨ (a.table.val = b.table.val ∧ a.index.val < b.index.val)

/-- `v` is the entries of the symbol tables among the first `upto`
sections: each an entry, in table order, none missing
(`reader_v::syms_ok`). -/
def SymsOk (d : List U8) (secs : List reader_v.SecHdr) (v : List reader_v.RawSym)
    (upto : Nat) : Prop :=
  (∀ s ∈ v, SymOk d secs s ∧ s.table.val < upto) ∧ v.Pairwise Before ∧
  (∀ t k, t < upto → ValidEntry d secs t k → ∃ s ∈ v, s.table.val = t ∧ s.index.val = k)

/-- The symbol's name bytes in the file are `name`'s
(`reader_v::name_of` equal to the name). -/
def NameIs (d : List U8) (s : reader_v.RawSym) (nm : List U8) : Prop :=
  s.name_end.val - s.name_start.val = nm.length ∧
  ∀ i, i < nm.length → d[s.name_start.val + i]! = nm[i]!

/-- The symbol asked for: of this name, and an object when `object`
(`reader_v::wanted`). -/
def Wanted (d : List U8) (s : reader_v.RawSym) (nm : List U8) (object : Bool) : Prop :=
  NameIs d s nm ∧ (object = true → s.info.bv &&& 15#8 = 1#8)

/-! ## The words -/

/-- **`u16_at`** returns for every input: the word when the file has two
bytes at the offset, nothing otherwise. -/
theorem u16_at_spec (d : Slice U8) (off : U64) :
    reader_v.u16_at d off ⦃ r =>
      (off.val + 2 ≤ d.val.length → ∃ v : U16, r = some v ∧ v.bv = le16 d.val off.val) ∧
      (d.val.length < off.val + 2 → r = none) ⦄ := by
  unfold reader_v.u16_at
  step as ⟨ n, hn ⟩
  have hnv : n.val = d.val.length := by rw [hn, cast_usize_u64, slice_len_val]
  split
  · rename_i h1
    have h1' : n.val < 2 := by scalar_tac
    rw [WP.spec_ok]
    exact ⟨fun h => absurd h (by omega), fun _ => rfl⟩
  · rename_i h1
    have h1' : 2 ≤ n.val := by scalar_tac
    step as ⟨ i1, hi1 ⟩
    split
    · rename_i h2
      have h2' : i1.val < off.val := by scalar_tac
      rw [WP.spec_ok]
      exact ⟨fun h => absurd h (by omega), fun _ => rfl⟩
    · rename_i h2
      have h2' : off.val ≤ i1.val := by scalar_tac
      have h3 : off.val + 2 ≤ d.val.length := by omega
      step as ⟨ i2, hi2 ⟩
      have hi2v : i2.val = off.val := by
        rw [hi2]
        exact cast_u64_usize off d.len (by rw [slice_len_val]; omega)
      have hb2 : i2.val < d.length := by scalar_tac
      step as ⟨ i3, hi3 ⟩
      step as ⟨ i4, hi4 ⟩
      step as ⟨ i5, hi5 ⟩
      have hb5 : i5.val < d.length := by scalar_tac
      step as ⟨ i6, hi6 ⟩
      step as ⟨ i7, hi7 ⟩
      step as ⟨ i8, hi8, hi8b ⟩
      step as ⟨ i9, hi9, hi9b ⟩
      refine ⟨fun _ => ⟨i9, rfl, ?_⟩, fun h => absurd h (by omega)⟩
      have c4 : i4.bv = i3.bv.setWidth 16 := by rw [hi4]; rfl
      have c7 : i7.bv = i6.bv.setWidth 16 := by rw [hi7]; rfl
      have e3 : d.val[off.val]! = i3 := bang_of_eq (by scalar_tac) hi3 hi2v
      have e6 : d.val[off.val + 1]! = i6 := bang_of_eq (by scalar_tac) hi6 (by omega)
      have e8 : i8.bv = i6.bv.setWidth 16 <<< (8 : Nat) :=
        hi8b.trans (congrArg (fun x : BitVec 16 => x <<< (8 : Nat)) c7)
      have e9 : i9.bv = i3.bv.setWidth 16 ||| (i6.bv.setWidth 16 <<< (8 : Nat)) :=
        hi9b.trans (congrArg₂ (· ||| ·) c4 e8)
      refine e9.trans ?_
      unfold le16
      rw [e3, e6]

/-- **`u32_at`** returns for every input: the word when the file has
four bytes at the offset, nothing otherwise. -/
theorem u32_at_spec (d : Slice U8) (off : U64) :
    reader_v.u32_at d off ⦃ r =>
      (off.val + 4 ≤ d.val.length → ∃ v : U32, r = some v ∧ v.bv = le32 d.val off.val) ∧
      (d.val.length < off.val + 4 → r = none) ⦄ := by
  unfold reader_v.u32_at
  step as ⟨ n, hn ⟩
  have hnv : n.val = d.val.length := by rw [hn, cast_usize_u64, slice_len_val]
  split
  · rename_i h1
    have h1' : n.val < 4 := by scalar_tac
    rw [WP.spec_ok]
    exact ⟨fun h => absurd h (by omega), fun _ => rfl⟩
  · rename_i h1
    have h1' : 4 ≤ n.val := by scalar_tac
    step as ⟨ i1, hi1 ⟩
    split
    · rename_i h2
      have h2' : i1.val < off.val := by scalar_tac
      rw [WP.spec_ok]
      exact ⟨fun h => absurd h (by omega), fun _ => rfl⟩
    · rename_i h2
      have h2' : off.val ≤ i1.val := by scalar_tac
      have h3 : off.val + 4 ≤ d.val.length := by omega
      step as ⟨ i2, hi2 ⟩
      have hi2v : i2.val = off.val := by
        rw [hi2]
        exact cast_u64_usize off d.len (by rw [slice_len_val]; omega)
      have hb2 : i2.val < d.length := by scalar_tac
      step as ⟨ i3, hi3 ⟩
      step as ⟨ i4, hi4 ⟩
      step as ⟨ i5, hi5 ⟩
      have hb5 : i5.val < d.length := by scalar_tac
      step as ⟨ i6, hi6 ⟩
      step as ⟨ i7, hi7 ⟩
      step as ⟨ i8, hi8, hi8b ⟩
      step as ⟨ i9, hi9, hi9b ⟩
      step as ⟨ i10, hi10 ⟩
      have hb10 : i10.val < d.length := by scalar_tac
      step as ⟨ i11, hi11 ⟩
      step as ⟨ i12, hi12 ⟩
      step as ⟨ i13, hi13, hi13b ⟩
      step as ⟨ i14, hi14, hi14b ⟩
      step as ⟨ i15, hi15 ⟩
      have hb15 : i15.val < d.length := by scalar_tac
      step as ⟨ i16, hi16 ⟩
      step as ⟨ i17, hi17 ⟩
      step as ⟨ i18, hi18, hi18b ⟩
      step as ⟨ i19, hi19, hi19b ⟩
      refine ⟨fun _ => ⟨i19, rfl, ?_⟩, fun h => absurd h (by omega)⟩
      have c4 : i4.bv = i3.bv.setWidth 32 := by rw [hi4]; rfl
      have c7 : i7.bv = i6.bv.setWidth 32 := by rw [hi7]; rfl
      have c12 : i12.bv = i11.bv.setWidth 32 := by rw [hi12]; rfl
      have c17 : i17.bv = i16.bv.setWidth 32 := by rw [hi17]; rfl
      have e3 : d.val[off.val]! = i3 := bang_of_eq (by scalar_tac) hi3 hi2v
      have e6 : d.val[off.val + 1]! = i6 := bang_of_eq (by scalar_tac) hi6 (by omega)
      have e11 : d.val[off.val + 2]! = i11 := bang_of_eq (by scalar_tac) hi11 (by omega)
      have e16 : d.val[off.val + 3]! = i16 := bang_of_eq (by scalar_tac) hi16 (by omega)
      have e8 : i8.bv = i6.bv.setWidth 32 <<< (8 : Nat) :=
        hi8b.trans (congrArg (fun x : BitVec 32 => x <<< (8 : Nat)) c7)
      have e9 : i9.bv = i3.bv.setWidth 32 ||| (i6.bv.setWidth 32 <<< (8 : Nat)) :=
        hi9b.trans (congrArg₂ (· ||| ·) c4 e8)
      have e13 : i13.bv = i11.bv.setWidth 32 <<< (16 : Nat) :=
        hi13b.trans (congrArg (fun x : BitVec 32 => x <<< (16 : Nat)) c12)
      have e14 := hi14b.trans (congrArg₂ (· ||| ·) e9 e13)
      have e18 : i18.bv = i16.bv.setWidth 32 <<< (24 : Nat) :=
        hi18b.trans (congrArg (fun x : BitVec 32 => x <<< (24 : Nat)) c17)
      have e19 := hi19b.trans (congrArg₂ (· ||| ·) e14 e18)
      refine e19.trans ?_
      unfold le32
      rw [e3, e6, e11, e16]

/-! ## Strings and extents -/

/-- **`str_end`** returns, and what it returns is the end of the string
that starts at `frm`. -/
theorem str_end_spec (d : Slice U8) (frm lim : Usize) (h1 : frm.val ≤ lim.val)
    (h2 : lim.val ≤ d.val.length) :
    reader_v.str_end d frm lim ⦃ r => IsStr d.val frm.val lim.val r.val ⦄ := by
  unfold reader_v.str_end reader_v.str_end_loop
  apply loop.spec_decr_nat (fun (i : Usize) => lim.val - i.val)
    (fun i => frm.val ≤ i.val ∧ i.val ≤ lim.val ∧
      ∀ j, frm.val ≤ j → j < i.val → d.val[j]! ≠ 0#u8)
  · intro i hi
    obtain ⟨ha, hb, hc⟩ := hi
    unfold reader_v.str_end_loop.body
    split
    · rename_i hlt
      have hlt' : i.val < lim.val := by scalar_tac
      have hbd : i.val < d.length := by scalar_tac
      step as ⟨ b, hb' ⟩
      have eb : d.val[i.val]! = b := bang_of_eq (by omega) hb' rfl
      split
      · rename_i hne
        simp only [bne_iff_ne, ne_eq] at hne
        step as ⟨ i2, hi2 ⟩
        refine ⟨by omega, by omega, ?_, by omega⟩
        intro j hj1 hj2
        by_cases hji : j = i.val
        · rw [hji, eb]; exact hne
        · exact hc j hj1 (by omega)
      · rename_i heq
        simp only [bne_iff_ne, ne_eq, not_not] at heq
        simp only [WP.spec_ok]
        unfold IsStr
        exact ⟨ha, hb, h2, hc, fun _ => by rw [eb]; exact heq⟩
    · rename_i hge
      have : i.val = lim.val := by scalar_tac
      simp only [WP.spec_ok]
      unfold IsStr
      exact ⟨ha, hb, h2, hc, fun h => absurd h (by omega)⟩
  · exact ⟨Nat.le_refl _, h1, fun j hj1 hj2 => absurd hj2 (by omega)⟩

/-- **`section_range`** returns for every input: the section's extent, or
nothing exactly when a section with file content lies outside the file. -/
theorem section_range_spec (len : Usize) (s : reader_v.SecHdr) :
    reader_v.section_range len s ⦃ r =>
      match r with
      | some (a, b) => SecRange len.val s a.val b.val
      | none => s.kind ≠ reader_v.SHT_NOBITS ∧ len.val < s.offset.val + s.size.val ⦄ := by
  unfold reader_v.section_range
  split
  · rename_i hk
    simp only [WP.spec_ok]
    unfold SecRange
    exact Or.inl ⟨hk, by scalar_tac, by scalar_tac⟩
  · rename_i hk
    step as ⟨ i, hi ⟩
    have hiv : i.val = s.offset.val := by rw [hi]; scalar_tac
    step as ⟨ i1, hi1 ⟩
    have hi1v : i1.val = s.size.val := by rw [hi1]; scalar_tac
    step as ⟨ e, he ⟩
    step as ⟨ i2, hi2 ⟩
    have hi2v : i2.val = len.val := by rw [hi2]; exact cast_usize_u64 len
    split
    · rename_i hgt
      have hgt' : i2.val < e.val := by scalar_tac
      simp only [WP.spec_ok]
      exact ⟨hk, by omega⟩
    · rename_i hle
      have hle' : e.val ≤ len.val := by scalar_tac
      step as ⟨ i3, hi3 ⟩
      have hi3v : i3.val = s.offset.val := by rw [hi3]; scalar_tac
      step as ⟨ i4, hi4 ⟩
      have hi4v : i4.val = e.val := by rw [hi4]; exact cast_u64_usize e len hle'
      unfold SecRange
      exact Or.inr ⟨hk, by omega, hi3v, by omega⟩

/-! ## The symbol tables -/

theorem info_spec (d : Slice U8) (b e : Usize) (h : b.val + e.val ≤ d.val.length) :
    (if e > 12#usize then do
        let i1 ← b + 12#usize
        d.index_usize i1
      else ok 0#u8) ⦃ info => 12 < e.val → info = d.val[b.val + 12]! ⦄ := by
  split
  · rename_i hgt
    have hgt' : 12 < e.val := by scalar_tac
    step as ⟨ i1, hi1 ⟩
    have hb1 : i1.val < d.length := by scalar_tac
    step as ⟨ x, hx ⟩
    try intro _
    exact (bang_of_eq (by scalar_tac) hx (by omega)).symm
  · rename_i hle
    simp only [WP.spec_ok]
    intro hc
    exact absurd hc (by scalar_tac)

theorem ent_spec (sh : reader_v.SecHdr) :
    (if sh.entsize = 0#u32 then ok 16#usize
      else ok (UScalar.cast UScalarTy.Usize sh.entsize)) ⦃ e => e.val = entSize sh ∧ 1 ≤ e.val ⦄ := by
  unfold entSize
  split
  · rename_i h
    simp only [WP.spec_ok]
    exact ⟨by scalar_tac, by scalar_tac⟩
  · rename_i h
    simp only [WP.spec_ok]
    exact ⟨by scalar_tac, by scalar_tac⟩

/-- **`sym_entry`** returns for every entry inside the file, and what it
returns holds the entry's fields. -/
theorem sym_entry_spec (d : Slice U8) (ss se b e t k : Usize) (h1 : ss.val ≤ se.val)
    (h2 : se.val ≤ d.val.length) (h3 : b.val + e.val ≤ d.val.length) :
    reader_v.sym_entry d ss se b e t k ⦃ r =>
      match r with
      | .Ok s => EntryOk d.val ss.val se.val b.val e.val s ∧ s.table = t ∧ s.index = k
      | .Err _ => True ⦄ := by
  unfold reader_v.sym_entry
  have hlen := slice_lt_u64 d
  split
  · elf_done
  rename_i he4
  have he4' : 4 ≤ e.val := by scalar_tac
  step as ⟨ i, hi ⟩
  have hiv : i.val = b.val := by rw [hi]; exact cast_usize_u64 b
  step with (u32_at_spec d i) as ⟨ o, ho1, ho2 ⟩
  rcases o with _ | v
  · elf_done
  try dsimp only
  obtain ⟨v', hv', hvb⟩ := ho1 (by omega)
  have hvv : v = v' := Option.some.inj hv'
  subst hvv
  rw [hiv] at hvb
  have hvn : v.val = (le32 d.val b.val).toNat := by rw [← hvb]; first | rfl | simp
  step with (info_spec d b e h3) as ⟨ info, hinfo ⟩
  step as ⟨ i1, hi1 ⟩
  have hi1v : i1.val = v.val := by rw [hi1]; scalar_tac
  step as ⟨ i2, hi2 ⟩
  step as ⟨ i3, hi3 ⟩
  have hi3v : i3.val = se.val - ss.val := by rw [hi3, cast_usize_u64]; omega
  split
  · step as ⟨ i4, hi4 ⟩
    all_goals (try elf_done)
  rename_i hns
  have hns' : v.val ≤ se.val - ss.val := by scalar_tac
  step as ⟨ i4, hi4 ⟩
  have hi4v : i4.val = v.val := by rw [hi4]; scalar_tac
  step as ⟨ name_start, hname ⟩
  step with (str_end_spec d name_start se (by omega) h2) as ⟨ name_end, hstr ⟩
  split
  · elf_done
  rename_i he8
  have he8' : 8 ≤ e.val := by scalar_tac
  step as ⟨ i5, hi5 ⟩
  have hi5v : i5.val = b.val := by rw [hi5]; exact cast_usize_u64 b
  step as ⟨ i6, hi6 ⟩
  step with (u32_at_spec d i6) as ⟨ o1, ho11, ho12 ⟩
  rcases o1 with _ | v1
  · elf_done
  try dsimp only
  obtain ⟨v1', hv1', hv1b⟩ := ho11 (by omega)
  have hvv1 : v1 = v1' := Option.some.inj hv1'
  subst hvv1
  rw [show i6.val = b.val + 4 by omega] at hv1b
  split
  · elf_done
  rename_i he12
  have he12' : 12 ≤ e.val := by scalar_tac
  step as ⟨ i7, hi7 ⟩
  have hi7v : i7.val = b.val := by rw [hi7]; exact cast_usize_u64 b
  step as ⟨ i8, hi8 ⟩
  step with (u32_at_spec d i8) as ⟨ o2, ho21, ho22 ⟩
  rcases o2 with _ | v2
  · elf_done
  try dsimp only
  obtain ⟨v2', hv2', hv2b⟩ := ho21 (by omega)
  have hvv2 : v2 = v2' := Option.some.inj hv2'
  subst hvv2
  rw [show i8.val = b.val + 8 by omega] at hv2b
  split
  · elf_done
  rename_i he16
  have he16' : 16 ≤ e.val := by scalar_tac
  step as ⟨ i9, hi9 ⟩
  have hi9v : i9.val = b.val := by rw [hi9]; exact cast_usize_u64 b
  step as ⟨ i10, hi10 ⟩
  step with (u16_at_spec d i10) as ⟨ o3, ho31, ho32 ⟩
  rcases o3 with _ | v3
  · elf_done
  try dsimp only
  obtain ⟨v3', hv3', hv3b⟩ := ho31 (by omega)
  have hvv3 : v3 = v3' := Option.some.inj hv3'
  subst hvv3
  rw [show i10.val = b.val + 14 by omega] at hv3b
  have hnsv : name_start.val = ss.val + (le32 d.val b.val).toNat := by omega
  rw [WP.spec_ok]
  refine ⟨?_, rfl, rfl⟩
  unfold EntryOk
  exact ⟨he16', hv1b, hv2b, hinfo (by omega), hv3b, hnsv, hstr⟩

theorem pw_before {t : Usize} : ∀ (l : List reader_v.RawSym), (∀ s ∈ l, s.table = t) →
    l.Pairwise (fun a b => a.index.val < b.index.val) → l.Pairwise Before
  | [], _, _ => List.Pairwise.nil
  | a :: l, hall, hp => by
    rw [List.pairwise_cons] at hp ⊢
    refine ⟨fun b hb => ?_,
      pw_before l (fun s hs => hall s (List.mem_cons_of_mem _ hs)) hp.2⟩
    unfold Before
    refine Or.inr ⟨?_, hp.1 b hb⟩
    rw [hall a (by simp), hall b (List.mem_cons_of_mem _ hb)]

/-- **`sym_entries`**: the entries of one table, appended: each an entry
of its index, in order, and every entry the table has room for. -/
theorem sym_entries_spec (d : Slice U8) (ss se ts te e t : Usize)
    (acc : alloc.vec.Vec reader_v.RawSym) (h1 : ss.val ≤ se.val) (h2 : se.val ≤ d.val.length)
    (h3 : ts.val ≤ te.val) (h4 : te.val ≤ d.val.length) (he : 1 ≤ e.val)
    (hroom : acc.val.length + (te.val - ts.val) < Usize.max) :
    reader_v.sym_entries d ss se ts te e t acc ⦃ r =>
      match r with
      | .Ok v => ∃ l : List reader_v.RawSym, v.val = acc.val ++ l ∧ l.length ≤ te.val - ts.val ∧
          (∀ s ∈ l, s.table = t ∧ ts.val + s.index.val * e.val + e.val ≤ te.val ∧
            EntryOk d.val ss.val se.val (ts.val + s.index.val * e.val) e.val s) ∧
          l.Pairwise (fun a b => a.index.val < b.index.val) ∧
          (∀ k, ts.val + k * e.val + e.val ≤ te.val → ∃ s ∈ l, s.index.val = k)
      | .Err _ => True ⦄ := by
  unfold reader_v.sym_entries reader_v.sym_entries_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec reader_v.RawSym × Usize × Usize) => te.val - s.2.1.val)
    (fun s => ∃ l : List reader_v.RawSym, s.1.val = acc.val ++ l ∧ l.length = s.2.2.val ∧
      s.2.1.val = ts.val + s.2.2.val * e.val ∧ s.2.1.val ≤ te.val ∧
      (∀ x ∈ l, x.table = t ∧ x.index.val < s.2.2.val ∧
        ts.val + x.index.val * e.val + e.val ≤ te.val ∧
        EntryOk d.val ss.val se.val (ts.val + x.index.val * e.val) e.val x) ∧
      l.Pairwise (fun a b => a.index.val < b.index.val) ∧
      (∀ j, j < s.2.2.val → ∃ x ∈ l, x.index.val = j))
  · intro s hs
    obtain ⟨out, pos, k⟩ := s
    obtain ⟨l, hout, hlen, hpos, hpte, hall, hpw, hcomp⟩ := hs
    dsimp only at hout hlen hpos hpte hall hpw hcomp ⊢
    unfold reader_v.sym_entries_loop.body
    have hkle : k.val * 1 ≤ k.val * e.val := Nat.mul_le_mul_left k.val he
    step as ⟨ i, hi ⟩
    split
    · rename_i hge
      have hge' : e.val ≤ te.val - pos.val := by scalar_tac
      step with (sym_entry_spec d ss se pos e t k h1 h2 (by omega)) as ⟨ r, hr ⟩
      rcases r with v | x
      · dsimp only at hr ⊢
        obtain ⟨hv, hvt, hvk⟩ := hr
        rw [hpos] at hv
        have hol : out.val.length = acc.val.length + k.val := by
          rw [hout, List.length_append, hlen]
        have hroom' : out.val.length < Usize.max := by omega
        step as ⟨ out1, hout1 ⟩
        step as ⟨ pos1, hpos1 ⟩
        step as ⟨ k1, hk1 ⟩
        have hmul : (k.val + 1) * e.val = k.val * e.val + e.val := by ring
        refine ⟨⟨l ++ [v], ?_, ?_, ?_, ?_, ?_, ?_, ?_⟩, ?_⟩
        · rw [hout1, hout, List.append_assoc]
        · rw [List.length_append, hlen, hk1]; rfl
        · rw [hpos1, hpos, hk1, hmul]; omega
        · omega
        · intro x hx
          rcases List.mem_append.1 hx with hx | hx
          · obtain ⟨a1, a2, a3, a4⟩ := hall x hx
            exact ⟨a1, by omega, a3, a4⟩
          · have hxv := List.mem_singleton.1 hx
            rw [hxv, hvk]
            exact ⟨hvt, by omega, by omega, hv⟩
        · rw [List.pairwise_append]
          refine ⟨hpw, List.pairwise_singleton _ _, ?_⟩
          intro a ha b hb
          have hbv := List.mem_singleton.1 hb
          rw [hbv, hvk]
          exact (hall a ha).2.1
        · intro j hj
          by_cases hjk : j < k.val
          · obtain ⟨x, hx, hxj⟩ := hcomp j hjk
            exact ⟨x, List.mem_append_left _ hx, hxj⟩
          · exact ⟨v, List.mem_append_right _ (List.mem_singleton.2 rfl), by rw [hvk]; omega⟩
        · omega
      · elf_done
    · rename_i hlt
      have hlt' : te.val - pos.val < e.val := by scalar_tac
      simp only [WP.spec_ok]
      refine ⟨l, hout, by omega, ?_, hpw, ?_⟩
      · intro x hx
        obtain ⟨a1, _, a3, a4⟩ := hall x hx
        exact ⟨a1, a3, a4⟩
      · intro j hj
        apply hcomp
        by_contra hc
        have := Nat.mul_le_mul_right e.val (show k.val ≤ j by omega)
        omega
  · exact ⟨[], by simp, by scalar_tac, by simp, h3, by simp, List.Pairwise.nil,
      fun j hj => absurd hj (by simp)⟩

/-- A section that is no symbol table has no entry. -/
theorem symsOk_skip {d : List U8} {secs : List reader_v.SecHdr} {v : List reader_v.RawSym}
    {t : Nat} (hok : SymsOk d secs v t) (hk : secs[t]!.kind ≠ reader_v.SHT_SYMTAB) :
    SymsOk d secs v (t + 1) := by
  obtain ⟨h1, h2, h3⟩ := hok
  unfold SymsOk
  refine ⟨fun s hs => ⟨(h1 s hs).1, by have := (h1 s hs).2; omega⟩, h2, ?_⟩
  intro tt k htt hv
  by_cases htlt : tt < t
  · exact h3 tt k htlt hv
  · have htt' : tt = t := by omega
    subst htt'
    obtain ⟨_, hkk, _⟩ := hv
    exact absurd hkk hk

/-- The entries of table `t` appended to the entries of the tables before
it are the entries of the tables up to it. -/
theorem symsOk_extend {d : List U8} {secs : List reader_v.SecHdr}
    {prev l : List reader_v.RawSym} {t : Usize} {ss se ts te e : Nat} {sh : reader_v.SecHdr}
    (hok : SymsOk d secs prev t.val) (ht : t.val < secs.length) (esh : secs[t.val]! = sh)
    (hk : sh.kind = reader_v.SHT_SYMTAB) (hl : sh.link.val < secs.length)
    (hs : SecRange d.length secs[sh.link.val]! ss se) (htr : SecRange d.length sh ts te)
    (he : e = entSize sh)
    (hall : ∀ s ∈ l, s.table = t ∧ ts + s.index.val * e + e ≤ te ∧
      EntryOk d ss se (ts + s.index.val * e) e s)
    (hpw : l.Pairwise (fun a b => a.index.val < b.index.val))
    (hcomp : ∀ k, ts + k * e + e ≤ te → ∃ s ∈ l, s.index.val = k) :
    SymsOk d secs (prev ++ l) (t.val + 1) := by
  subst he
  obtain ⟨h1, h2, h3⟩ := hok
  unfold SymsOk
  refine ⟨?_, ?_, ?_⟩
  · intro s hs'
    rcases List.mem_append.1 hs' with hm | hm
    · exact ⟨(h1 s hm).1, by have := (h1 s hm).2; omega⟩
    · obtain ⟨a1, a2, a3⟩ := hall s hm
      have hst : s.table.val = t.val := by rw [a1]
      refine ⟨?_, by omega⟩
      unfold SymOk
      rw [hst, esh]
      exact ⟨ht, hk, hl, ss, se, ts, te, hs, htr, a2, a3⟩
  · rw [List.pairwise_append]
    refine ⟨h2, pw_before l (fun s hs' => (hall s hs').1) hpw, ?_⟩
    intro a ha b hb
    unfold Before
    exact Or.inl (by rw [(hall b hb).1]; exact (h1 a ha).2)
  · intro tt k htt hv
    by_cases htlt : tt < t.val
    · obtain ⟨s, hs', e1, e2⟩ := h3 tt k htlt hv
      exact ⟨s, List.mem_append_left _ hs', e1, e2⟩
    · have htt' : tt = t.val := by omega
      subst htt'
      obtain ⟨_, _, _, ss', se', ts', te', r1, r2, hle⟩ := hv
      rw [esh] at r2 hle
      obtain ⟨x1, x2⟩ := SecRange.unique r2 htr
      subst x1 x2
      obtain ⟨s, hs', hsk⟩ := hcomp k hle
      exact ⟨s, List.mem_append_right _ hs', by rw [(hall s hs').1], hsk⟩

/-- **`symbols`** returns, and what it returns is the entries of every
symbol table: each an entry, in table order, none missing. -/
theorem symbols_spec (d : Slice U8) (secs : alloc.vec.Vec reader_v.SecHdr)
    (hroom : secs.val.length * d.val.length < Usize.max) :
    reader_v.symbols d secs ⦃ r =>
      match r with
      | .Ok v => SymsOk d.val secs.val v.val secs.val.length
      | .Err _ => True ⦄ := by
  unfold reader_v.symbols reader_v.symbols_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec reader_v.RawSym × Usize) => secs.val.length - s.2.val)
    (fun s => s.2.val ≤ secs.val.length ∧ SymsOk d.val secs.val s.1.val s.2.val ∧
      s.1.val.length ≤ s.2.val * d.val.length)
  · intro s hs
    obtain ⟨out, t⟩ := s
    obtain ⟨htl, hok, hsz⟩ := hs
    dsimp only at htl hok hsz ⊢
    unfold reader_v.symbols_loop.body
    try dsimp only
    have hsucc : (t.val + 1) * d.val.length = t.val * d.val.length + d.val.length := by ring
    split
    · rename_i hlt
      have hlt' : t.val < secs.val.length := by scalar_tac
      have hmulsz : (t.val + 1) * d.val.length ≤ secs.val.length * d.val.length :=
        Nat.mul_le_mul_right d.val.length (by omega)
      step as ⟨ sh, hsh ⟩
      have esh : secs.val[t.val]! = sh := bang_of_eq hlt' hsh rfl
      split
      · rename_i hk
        step as ⟨ link, hlink ⟩
        have hlinkv : link.val = sh.link.val := by rw [hlink]; scalar_tac
        split
        · elf_done
        rename_i hll
        have hll' : link.val < secs.val.length := by scalar_tac
        step as ⟨ sh1, hsh1 ⟩
        have esh1 : secs.val[sh.link.val]! = sh1 := bang_of_eq hll' hsh1 hlinkv
        step with (section_range_spec d.len sh1) as ⟨ o, ho ⟩
        rcases o with _ | ⟨ss, se⟩
        · elf_done
        dsimp only at ho ⊢
        step with (section_range_spec d.len sh) as ⟨ o1, ho1 ⟩
        rcases o1 with _ | ⟨ts, te⟩
        · elf_done
        dsimp only at ho1 ⊢
        rw [slice_len_val] at ho ho1
        obtain ⟨hs1, hs2⟩ := ho.le
        obtain ⟨ht1, ht2⟩ := ho1.le
        step with (ent_spec sh) as ⟨ e, he, he1 ⟩
        step with (sym_entries_spec d ss se ts te e t out hs1 hs2 ht1 ht2 he1 (by omega))
          as ⟨ r, hr ⟩
        rcases r with v | x
        · dsimp only at hr ⊢
          obtain ⟨l, hv, hll2, hall, hpw, hcomp⟩ := hr
          step as ⟨ t1, ht1v ⟩
          refine ⟨by omega, ?_, ?_, by omega⟩
          · rw [hv, ht1v]
            exact symsOk_extend hok hlt' esh hk (by omega) (by rw [esh1]; exact ho) ho1 he
              hall hpw hcomp
          · rw [hv, List.length_append, ht1v]; omega
        · elf_done
      · rename_i hk
        step as ⟨ t1, ht1v ⟩
        refine ⟨by omega, ?_, ?_, by omega⟩
        · rw [ht1v]
          exact symsOk_skip hok (by rw [esh]; exact hk)
        · rw [ht1v]; omega
    · rename_i hge
      have hte : t.val = secs.val.length := by scalar_tac
      simp only [WP.spec_ok]
      rw [← hte]
      exact hok
  · have h0 : SymsOk d.val secs.val [] 0 := by
      unfold SymsOk
      exact ⟨fun s hs => absurd hs (by simp), List.Pairwise.nil,
        fun t k ht => absurd ht (by omega)⟩
    exact ⟨by scalar_tac, by simpa [alloc.vec.Vec.new] using h0, by simp [alloc.vec.Vec.new]⟩

/-! ## The lookup -/

theorem name_is_loop_spec (d : Slice U8) (ns : Usize) (nm : Slice U8)
    (h : ns.val + nm.val.length ≤ d.val.length) :
    reader_v.name_is_loop d ns nm 0#usize ⦃ r =>
      r = true ↔ ∀ i, i < nm.val.length → d.val[ns.val + i]! = nm.val[i]! ⦄ := by
  unfold reader_v.name_is_loop
  apply loop.spec_decr_nat (fun (i : Usize) => nm.val.length - i.val)
    (fun i => i.val ≤ nm.val.length ∧ ∀ j, j < i.val → d.val[ns.val + j]! = nm.val[j]!)
  · intro i hi
    obtain ⟨h1, h2⟩ := hi
    unfold reader_v.name_is_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : i.val < nm.val.length := by scalar_tac
      step as ⟨ i3, hi3 ⟩
      have hb3 : i3.val < d.length := by scalar_tac
      step as ⟨ x, hx ⟩
      have hbi : i.val < nm.length := by scalar_tac
      step as ⟨ y, hy ⟩
      have ex : d.val[ns.val + i.val]! = x := bang_of_eq (by scalar_tac) hx hi3
      have ey : nm.val[i.val]! = y := bang_of_eq hlt' hy rfl
      split
      · rename_i hne
        simp only [bne_iff_ne, ne_eq] at hne
        simp only [WP.spec_ok]
        constructor
        · intro hc; cases hc
        · intro hall
          have := hall i.val hlt'
          rw [ex, ey] at this
          exact absurd this hne
      · rename_i heq
        simp only [bne_iff_ne, ne_eq, not_not] at heq
        step as ⟨ i6, hi6 ⟩
        refine ⟨by omega, ?_, by omega⟩
        intro j hj
        by_cases hji : j = i.val
        · rw [hji, ex, ey]; exact heq
        · exact h2 j (by omega)
    · rename_i hge
      have : i.val = nm.val.length := by scalar_tac
      simp only [WP.spec_ok]
      constructor
      · intro _ j hj; exact h2 j (by omega)
      · intro _; first | rfl | trivial
  · exact ⟨by scalar_tac, fun j hj => absurd hj (by simp)⟩

/-- **`name_is`** returns, and answers `true` exactly when the symbol's
name bytes in the file are the name's. -/
theorem name_is_spec (d : Slice U8) (s : reader_v.RawSym) (nm : Slice U8)
    (h1 : s.name_start.val ≤ s.name_end.val) (h2 : s.name_end.val ≤ d.val.length) :
    reader_v.name_is d s nm ⦃ r => r = true ↔ NameIs d.val s nm.val ⦄ := by
  unfold reader_v.name_is
  step as ⟨ i, hi ⟩
  try dsimp only
  split
  · rename_i hne
    simp only [bne_iff_ne, ne_eq] at hne
    have hne' : i.val ≠ nm.val.length := by scalar_tac
    simp only [WP.spec_ok]
    constructor
    · intro hc; cases hc
    · intro hn
      obtain ⟨hn1, _⟩ := hn
      exact absurd hn1 (by omega)
  · rename_i heq
    simp only [bne_iff_ne, ne_eq, not_not] at heq
    have heq' : i.val = nm.val.length := by scalar_tac
    refine WP.spec_mono (name_is_loop_spec d s.name_start nm (by omega)) ?_
    intro r hr
    constructor
    · intro hc
      unfold NameIs
      exact ⟨by omega, hr.1 hc⟩
    · intro hn
      obtain ⟨_, hn2⟩ := hn
      exact hr.2 hn2

/-- **`find`** returns: the index of the first of the symbols that is
wanted, or nothing when none is. -/
theorem find_spec (d : Slice U8) (syms : alloc.vec.Vec reader_v.RawSym) (nm : Slice U8)
    (object : Bool)
    (hsy : ∀ s ∈ syms.val, s.name_start.val ≤ s.name_end.val ∧ s.name_end.val ≤ d.val.length) :
    reader_v.find d syms nm object ⦃ r =>
      match r with
      | some i => i.val < syms.val.length ∧ Wanted d.val syms.val[i.val]! nm.val object ∧
          ∀ j, j < i.val → ¬ Wanted d.val syms.val[j]! nm.val object
      | none => ∀ j, j < syms.val.length → ¬ Wanted d.val syms.val[j]! nm.val object ⦄ := by
  unfold reader_v.find reader_v.find_loop
  apply loop.spec_decr_nat (fun (i : Usize) => syms.val.length - i.val)
    (fun i => i.val ≤ syms.val.length ∧
      ∀ j, j < i.val → ¬ Wanted d.val syms.val[j]! nm.val object)
  · intro i hi
    obtain ⟨h1, h2⟩ := hi
    unfold reader_v.find_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : i.val < syms.val.length := by scalar_tac
      have hext : ∀ i1 : Usize, i1.val = i.val + 1 →
          ¬ Wanted d.val syms.val[i.val]! nm.val object →
          i1.val ≤ syms.val.length ∧
          (∀ j, j < i1.val → ¬ Wanted d.val syms.val[j]! nm.val object) ∧
          syms.val.length - i1.val < syms.val.length - i.val := by
        intro i1 hi1 hnw
        refine ⟨by omega, ?_, by omega⟩
        intro j hj
        by_cases hji : j = i.val
        · rw [hji]; exact hnw
        · exact h2 j (by omega)
      split
      · rename_i hobj
        step as ⟨ rs, hrs ⟩
        have ers : syms.val[i.val]! = rs := bang_of_eq hlt' hrs rfl
        have hmem : rs ∈ syms.val := by rw [hrs]; exact List.getElem_mem _
        step as ⟨ i2, hi2, hi2b ⟩
        have hcond : i2 = 1#u8 ↔ rs.info.bv &&& 15#8 = 1#8 := by
          rw [U8.eq_equiv_bv_eq]
          constructor
          · intro h; exact hi2b.symm.trans h
          · intro h; exact hi2b.trans h
        split
        · rename_i hone
          step with (name_is_spec d rs nm (hsy rs hmem).1 (hsy rs hmem).2) as ⟨ b, hb ⟩
          split
          · rename_i hbt
            have hw : Wanted d.val rs nm.val object := by
              unfold Wanted
              exact ⟨hb.1 hbt, fun _ => hcond.1 hone⟩
            simp only [WP.spec_ok]
            rw [ers]
            exact ⟨hlt', hw, h2⟩
          · rename_i hbf
            step as ⟨ i3, hi3 ⟩
            exact hext i3 hi3 (by
              rw [ers]; intro hw; obtain ⟨w1, _⟩ := hw; exact hbf (hb.2 w1))
        · rename_i hone
          step as ⟨ i3, hi3 ⟩
          exact hext i3 hi3 (by
            rw [ers]; intro hw; obtain ⟨_, w2⟩ := hw; exact hone (hcond.2 (w2 hobj)))
      · rename_i hobj
        step as ⟨ rs, hrs ⟩
        have ers : syms.val[i.val]! = rs := bang_of_eq hlt' hrs rfl
        have hmem : rs ∈ syms.val := by rw [hrs]; exact List.getElem_mem _
        step with (name_is_spec d rs nm (hsy rs hmem).1 (hsy rs hmem).2) as ⟨ b, hb ⟩
        split
        · rename_i hbt
          have hw : Wanted d.val rs nm.val object := by
            unfold Wanted
            exact ⟨hb.1 hbt, fun ho => absurd ho hobj⟩
          simp only [WP.spec_ok]
          rw [ers]
          exact ⟨hlt', hw, h2⟩
        · rename_i hbf
          step as ⟨ i3, hi3 ⟩
          exact hext i3 hi3 (by
            rw [ers]; intro hw; obtain ⟨w1, _⟩ := hw; exact hbf (hb.2 w1))
    · rename_i hge
      have : i.val = syms.val.length := by scalar_tac
      simp only [WP.spec_ok]
      intro j hj
      exact h2 j (by omega)
  · exact ⟨by scalar_tac, fun j hj => absurd hj (by simp)⟩

/-- Two entries of one index in one table have one name and one info. -/
theorem symOk_same {d : List U8} {secs : List reader_v.SecHdr} {a b : reader_v.RawSym}
    (ha : SymOk d secs a) (hb : SymOk d secs b) (ht : a.table.val = b.table.val)
    (hi : a.index.val = b.index.val) :
    a.name_start.val = b.name_start.val ∧ a.name_end.val = b.name_end.val ∧ a.info = b.info := by
  obtain ⟨_, _, _, ss, se, ts, te, r1, r2, _, ea⟩ := ha
  obtain ⟨_, _, _, ss', se', ts', te', r1', r2', _, eb⟩ := hb
  rw [← ht] at r1' r2' eb
  rw [← hi] at eb
  obtain ⟨x1, x2⟩ := SecRange.unique r1' r1
  obtain ⟨y1, y2⟩ := SecRange.unique r2' r2
  subst x1 x2 y1 y2
  obtain ⟨_, _, _, ia, _, sa, stra⟩ := ea
  obtain ⟨_, _, _, ib, _, sb, strb⟩ := eb
  have hns : a.name_start.val = b.name_start.val := sa.trans sb.symm
  rw [← hns] at strb
  exact ⟨hns, IsStr.unique stra strb, ia.trans ib.symm⟩

/-- **`lookup`** (the lookup the Config_Checker finds its images by)
returns. `Ok (some s)`: `s` is an entry of a symbol table, with that
entry's value, size, info and section index; its name bytes in the string
table are exactly the name asked for, and it is an object when one was
asked for; no entry before it in table order is such. `Ok none`: no entry
of any symbol table is such. -/
theorem lookup_spec (d : Slice U8) (secs : alloc.vec.Vec reader_v.SecHdr) (nm : Slice U8)
    (object : Bool) (hroom : secs.val.length * d.val.length < Usize.max) :
    reader_v.lookup d secs nm object ⦃ r =>
      match r with
      | .Ok (some s) => SymOk d.val secs.val s ∧ Wanted d.val s nm.val object ∧
          ∀ o, SymOk d.val secs.val o → Wanted d.val o nm.val object → ¬ Before o s
      | .Ok none => ∀ o, SymOk d.val secs.val o → ¬ Wanted d.val o nm.val object
      | .Err _ => True ⦄ := by
  unfold reader_v.lookup
  step with (symbols_spec d secs hroom) as ⟨ r, hr ⟩
  rcases r with v | x
  swap
  · elf_done
  dsimp only at hr ⊢
  obtain ⟨h1, h2, h3⟩ := hr
  have hbounds : ∀ s ∈ v.val,
      s.name_start.val ≤ s.name_end.val ∧ s.name_end.val ≤ d.val.length := by
    intro s hs
    obtain ⟨_, _, _, ss, se, ts, te, _, _, _, ea⟩ := (h1 s hs).1
    obtain ⟨_, _, _, _, _, _, a1, a2, a3, _, _⟩ := ea
    exact ⟨a1, by omega⟩
  have hfound : ∀ o, SymOk d.val secs.val o → ∃ j, j < v.val.length ∧
      (Wanted d.val o nm.val object ↔ Wanted d.val v.val[j]! nm.val object) ∧
      v.val[j]!.table.val = o.table.val ∧ v.val[j]!.index.val = o.index.val := by
    intro o ho
    have hval : o.table.val < secs.val.length ∧
        ValidEntry d.val secs.val o.table.val o.index.val := by
      obtain ⟨c1, c2, c3, ss, se, ts, te, r1, r2, c4, _⟩ := ho
      refine ⟨c1, ?_⟩
      unfold ValidEntry
      exact ⟨c1, c2, c3, ss, se, ts, te, r1, r2, c4⟩
    obtain ⟨s, hs, e1, e2⟩ := h3 o.table.val o.index.val hval.1 hval.2
    obtain ⟨j, hj, hjs⟩ := List.mem_iff_getElem.1 hs
    have ej : v.val[j]! = s := by rw [← hjs]; exact getElem!_pos v.val j hj
    obtain ⟨n1, n2, n3⟩ := symOk_same (h1 s hs).1 ho e1 e2
    refine ⟨j, hj, ?_, by rw [ej]; exact e1, by rw [ej]; exact e2⟩
    rw [ej]
    unfold Wanted NameIs
    rw [n1, n2, n3]
  step with (find_spec d v nm object hbounds) as ⟨ found, hf ⟩
  rcases found with _ | i
  · dsimp only at hf ⊢
    simp only [WP.spec_ok]
    intro o ho hw
    obtain ⟨j, hj, hiff, _, _⟩ := hfound o ho
    exact hf j hj (hiff.1 hw)
  · dsimp only at hf ⊢
    obtain ⟨hi, hwi, hfirst⟩ := hf
    step as ⟨ rs, hrs ⟩
    have ers : v.val[i.val]! = rs := bang_of_eq hi hrs rfl
    have hmem : rs ∈ v.val := by rw [hrs]; exact List.getElem_mem _
    refine ⟨(h1 rs hmem).1, by rw [← ers]; exact hwi, ?_⟩
    intro o ho hw hbef
    obtain ⟨j, hj, hiff, et, ei⟩ := hfound o ho
    rcases Nat.lt_trichotomy j i.val with hlt | heq | hgt
    · exact hfirst j hlt (hiff.1 hw)
    · rw [heq, ers] at et ei
      rcases hbef with hb | ⟨hb1, hb2⟩ <;> omega
    · have hp := (List.pairwise_iff_getElem.1 h2) i.val j hi hj hgt
      rw [← getElem!_pos v.val i.val hi, ← getElem!_pos v.val j hj, ers] at hp
      rcases hp with hp | ⟨hp1, hp2⟩ <;> rcases hbef with hb | ⟨hb1, hb2⟩ <;> omega

/-! ## Bytes at a virtual address -/

/-- The section holds the address: allocated, not empty, the address in
it (`reader_v::sec_holds`). -/
def SecHolds (s : reader_v.SecHdr) (addr : U32) : Prop :=
  s.flags.bv &&& reader_v.SHF_ALLOC.bv ≠ 0#32 ∧ 0 < s.size.val ∧ s.addr.val ≤ addr.val ∧
  addr.val < s.addr.val + s.size.val

/-- `v` is the `len` bytes at `addr` of section `s`: the file's bytes
from the section's offset plus the address's offset in it, zeros for a
section without file content (`reader_v::bytes_at`). -/
def BytesAt (d : List U8) (s : reader_v.SecHdr) (addr : U32) (len : Nat) (v : List U8) : Prop :=
  addr.val + len ≤ s.addr.val + s.size.val ∧
  if s.kind = reader_v.SHT_NOBITS then v = List.replicate len 0#u8
  else s.offset.val + (addr.val - s.addr.val) + len ≤ d.length ∧
    v = (d.drop (s.offset.val + (addr.val - s.addr.val))).take len

/-- **`section_at`** returns: the first section that holds the address,
or nothing when none does. -/
theorem section_at_spec (secs : alloc.vec.Vec reader_v.SecHdr) (addr : U32) :
    reader_v.section_at secs addr ⦃ r =>
      match r with
      | some i => i.val < secs.val.length ∧ SecHolds secs.val[i.val]! addr ∧
          ∀ j, j < i.val → ¬ SecHolds secs.val[j]! addr
      | none => ∀ j, j < secs.val.length → ¬ SecHolds secs.val[j]! addr ⦄ := by
  unfold reader_v.section_at reader_v.section_at_loop
  apply loop.spec_decr_nat (fun (i : Usize) => secs.val.length - i.val)
    (fun i => i.val ≤ secs.val.length ∧ ∀ j, j < i.val → ¬ SecHolds secs.val[j]! addr)
  · intro i hi
    obtain ⟨h1, h2⟩ := hi
    unfold reader_v.section_at_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : i.val < secs.val.length := by scalar_tac
      have hext : ∀ i1 : Usize, i1.val = i.val + 1 → ¬ SecHolds secs.val[i.val]! addr →
          i1.val ≤ secs.val.length ∧ (∀ j, j < i1.val → ¬ SecHolds secs.val[j]! addr) ∧
          secs.val.length - i1.val < secs.val.length - i.val := by
        intro i1 hi1 hnw
        refine ⟨by omega, ?_, by omega⟩
        intro j hj
        by_cases hji : j = i.val
        · rw [hji]; exact hnw
        · exact h2 j (by omega)
      step as ⟨ sh, hsh ⟩
      have esh : secs.val[i.val]! = sh := bang_of_eq hlt' hsh rfl
      step as ⟨ i2, hi2, hi2b ⟩
      have hz : i2 = 0#u32 ↔ sh.flags.bv &&& reader_v.SHF_ALLOC.bv = 0#32 := by
        rw [U32.eq_equiv_bv_eq]
        constructor
        · intro h; exact hi2b.symm.trans h
        · intro h; exact hi2b.trans h
      split
      · rename_i hnz
        simp only [bne_iff_ne, ne_eq] at hnz
        split
        · rename_i hsz
          have hsz' : 0 < sh.size.val := by scalar_tac
          split
          · rename_i hge
            have hge' : sh.addr.val ≤ addr.val := by scalar_tac
            step as ⟨ i3, hi3 ⟩
            have hi3v : i3.val = addr.val := by rw [hi3]; scalar_tac
            step as ⟨ i4, hi4 ⟩
            have hi4v : i4.val = sh.addr.val := by rw [hi4]; scalar_tac
            step as ⟨ i5, hi5 ⟩
            have hi5v : i5.val = sh.size.val := by rw [hi5]; scalar_tac
            step as ⟨ i6, hi6 ⟩
            split
            · rename_i hin
              have hin' : i3.val < i6.val := by scalar_tac
              have hh : SecHolds sh addr := by
                unfold SecHolds
                exact ⟨fun hc => hnz (hz.2 hc), hsz', hge', by omega⟩
              simp only [WP.spec_ok]
              rw [esh]
              exact ⟨hlt', hh, h2⟩
            · rename_i hout
              have hout' : ¬ i3.val < i6.val := by scalar_tac
              step as ⟨ i7, hi7 ⟩
              exact hext i7 hi7 (by
                rw [esh]; intro hh; obtain ⟨_, _, _, a4⟩ := hh; omega)
          · rename_i hlt2
            have hlt2' : ¬ sh.addr.val ≤ addr.val := by scalar_tac
            step as ⟨ i3, hi3 ⟩
            exact hext i3 hi3 (by
              rw [esh]; intro hh; obtain ⟨_, _, a3, _⟩ := hh; omega)
        · rename_i hsz
          have hsz' : ¬ 0 < sh.size.val := by scalar_tac
          step as ⟨ i3, hi3 ⟩
          exact hext i3 hi3 (by
            rw [esh]; intro hh; obtain ⟨_, a2, _, _⟩ := hh; omega)
      · rename_i hz0
        simp only [bne_iff_ne, ne_eq, not_not] at hz0
        step as ⟨ i3, hi3 ⟩
        exact hext i3 hi3 (by
          rw [esh]; intro hh; obtain ⟨a1, _, _, _⟩ := hh; exact a1 (hz.1 hz0))
    · rename_i hge
      have hin : i.val = secs.val.length := by scalar_tac
      simp only [WP.spec_ok]
      intro j hj
      exact h2 j (by omega)
  · exact ⟨by scalar_tac, fun j hj => absurd hj (by simp)⟩

/-- **`copy_range`** returns the `len` bytes of the file from `start`. -/
theorem copy_range_spec (d : Slice U8) (start len : Usize)
    (h : start.val + len.val ≤ d.val.length) :
    reader_v.copy_range d start len ⦃ r => r.val = (d.val.drop start.val).take len.val ⦄ := by
  unfold reader_v.copy_range reader_v.copy_range_loop
  apply loop.spec_decr_nat (fun (s : alloc.vec.Vec U8 × Usize) => len.val - s.2.val)
    (fun s => s.2.val ≤ len.val ∧ s.1.val = (d.val.drop start.val).take s.2.val)
  · intro s hs
    obtain ⟨out, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold reader_v.copy_range_loop.body
    split
    · rename_i hlt
      have hlt' : i.val < len.val := by scalar_tac
      step as ⟨ i1, hi1 ⟩
      have hb1 : i1.val < d.length := by scalar_tac
      step as ⟨ x, hx ⟩
      have hdl : i.val < (d.val.drop start.val).length := by
        rw [List.length_drop]; omega
      have hol : out.val.length = i.val := by
        rw [h2, List.length_take]; omega
      have hroom : out.val.length < Usize.max := by scalar_tac
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i3, hi3 ⟩
      have hx' : (d.val.drop start.val)[i.val] = x := by
        rw [hx]; simp [hi1]
      refine ⟨by omega, ?_, by omega⟩
      rw [hout1, h2, hi3, List.take_succ_eq_append_getElem hdl, hx']
    · rename_i hge
      have hin : i.val = len.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [h2, hin]
  · exact ⟨by scalar_tac, by simp [alloc.vec.Vec.new]⟩

/-- **`zeros`** returns `len` zero bytes. -/
theorem zeros_spec (len : Usize) :
    reader_v.zeros len ⦃ r => r.val = List.replicate len.val 0#u8 ⦄ := by
  unfold reader_v.zeros reader_v.zeros_loop
  apply loop.spec_decr_nat (fun (s : alloc.vec.Vec U8 × Usize) => len.val - s.2.val)
    (fun s => s.2.val ≤ len.val ∧ s.1.val = List.replicate s.2.val 0#u8)
  · intro s hs
    obtain ⟨out, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold reader_v.zeros_loop.body
    split
    · rename_i hlt
      have hlt' : i.val < len.val := by scalar_tac
      have hol : out.val.length = i.val := by rw [h2, List.length_replicate]
      have hroom : out.val.length < Usize.max := by scalar_tac
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨by omega, ?_, by omega⟩
      rw [hout1, h2, hi1, List.replicate_succ']
    · rename_i hge
      have hin : i.val = len.val := by scalar_tac
      simp only [WP.spec_ok]
      rw [h2, hin]
  · exact ⟨by scalar_tac, by simp [alloc.vec.Vec.new]⟩

/-- **`read_at`** returns for every input. `Ok v`: there is a first
allocated, non-empty section holding the address, the range ends inside
it, and `v` is the file's bytes at the section's offset plus the
address's offset within the section (zeros for a section without file
content). The refusal `NoSection` is given only when no section holds the
address. -/
theorem read_at_spec (d : Slice U8) (secs : alloc.vec.Vec reader_v.SecHdr) (addr : U32)
    (len : Usize) :
    reader_v.read_at d secs addr len ⦃ r =>
      match r with
      | .Ok v => ∃ i, i < secs.val.length ∧ SecHolds secs.val[i]! addr ∧
          (∀ j, j < i → ¬ SecHolds secs.val[j]! addr) ∧
          BytesAt d.val secs.val[i]! addr len.val v.val
      | .Err e => e = reader_v.AtErr.NoSection →
          ∀ j, j < secs.val.length → ¬ SecHolds secs.val[j]! addr ⦄ := by
  unfold reader_v.read_at
  step with (section_at_spec secs addr) as ⟨ o, ho ⟩
  rcases o with _ | i
  · dsimp only at ho ⊢
    rw [WP.spec_ok]
    intro _
    exact ho
  dsimp only at ho ⊢
  obtain ⟨hi, hh, hfirst⟩ := ho
  step as ⟨ s, hs ⟩
  have es : secs.val[i.val]! = s := bang_of_eq hi hs rfl
  rw [es] at hh
  have hh' := hh
  obtain ⟨_, a2, a3, a4⟩ := hh'
  step as ⟨ i1, hi1 ⟩
  have hi1v : i1.val = s.addr.val := by rw [hi1]; scalar_tac
  step as ⟨ i2, hi2 ⟩
  have hi2v : i2.val = s.size.val := by rw [hi2]; scalar_tac
  step as ⟨ i3, hi3 ⟩
  step as ⟨ i4, hi4 ⟩
  have hi4v : i4.val = addr.val := by rw [hi4]; scalar_tac
  step as ⟨ room, hroom ⟩
  step as ⟨ i5, hi5 ⟩
  have hi5v : i5.val = len.val := by rw [hi5]; exact cast_usize_u64 len
  split
  · rw [WP.spec_ok]
    intro hc
    cases hc
  rename_i hfits
  have hfits' : len.val ≤ room.val := by scalar_tac
  split
  · rename_i hk
    step with (zeros_spec len) as ⟨ v, hv ⟩
    refine ⟨i.val, hi, by rw [es]; exact hh, hfirst, ?_⟩
    rw [es]
    unfold BytesAt
    refine ⟨by omega, ?_⟩
    rw [if_pos hk]
    exact hv
  · rename_i hk
    step as ⟨ i6, hi6 ⟩
    have hi6v : i6.val = s.offset.val := by rw [hi6]; scalar_tac
    step as ⟨ i7, hi7 ⟩
    step as ⟨ i8, hi8 ⟩
    have hi8v : i8.val = i7.val := by rw [hi8]; scalar_tac
    step as ⟨ start, hstart ⟩
    try dsimp only
    step as ⟨ n, hn ⟩
    have hnv : n.val = d.val.length := by rw [hn, cast_usize_u64, slice_len_val]
    step as ⟨ i10, hi10 ⟩
    have hi10v : i10.val = len.val := by rw [hi10]; exact cast_usize_u64 len
    split
    · rw [WP.spec_ok]
      intro hc
      cases hc
    rename_i hle
    have hle' : len.val ≤ n.val := by scalar_tac
    step as ⟨ i11, hi11 ⟩
    have hi11v : i11.val = len.val := by rw [hi11]; exact cast_usize_u64 len
    step as ⟨ i12, hi12 ⟩
    split
    · rw [WP.spec_ok]
      intro hc
      cases hc
    rename_i hst
    have hst' : start.val ≤ i12.val := by scalar_tac
    step as ⟨ i13, hi13 ⟩
    have hi13v : i13.val = start.val := by
      rw [hi13]
      exact cast_u64_usize start d.len (by rw [slice_len_val]; omega)
    have hsv : start.val = s.offset.val + (addr.val - s.addr.val) := by omega
    step with (copy_range_spec d i13 len (by omega)) as ⟨ v, hv ⟩
    refine ⟨i.val, hi, by rw [es]; exact hh, hfirst, ?_⟩
    rw [es]
    unfold BytesAt
    refine ⟨by omega, ?_⟩
    rw [if_neg hk]
    refine ⟨by omega, ?_⟩
    rw [hv, hi13v, hsv]

/-! ## The headers -/

theorem rd_segment_total (d : Slice U8) (base : U64) (h : base.val ≤ 8589934592) :
    reader_v.rd_segment d base ⦃ _ => True ⦄ := by
  unfold reader_v.rd_segment
  step with (u32_at_spec d base) as ⟨ o0 ⟩
  rcases o0 with _ | w0
  · elf_done
  try dsimp only
  step as ⟨ b1, hb1 ⟩
  step with (u32_at_spec d b1) as ⟨ o1 ⟩
  rcases o1 with _ | w1
  · elf_done
  try dsimp only
  step as ⟨ b2, hb2 ⟩
  step with (u32_at_spec d b2) as ⟨ o2 ⟩
  rcases o2 with _ | w2
  · elf_done
  try dsimp only
  step as ⟨ b3, hb3 ⟩
  step with (u32_at_spec d b3) as ⟨ o3 ⟩
  rcases o3 with _ | w3
  · elf_done
  try dsimp only
  step as ⟨ b4, hb4 ⟩
  step with (u32_at_spec d b4) as ⟨ o4 ⟩
  rcases o4 with _ | w4
  · elf_done
  try dsimp only
  step as ⟨ b5, hb5 ⟩
  step with (u32_at_spec d b5) as ⟨ o5 ⟩
  rcases o5 with _ | w5
  · elf_done
  try dsimp only
  step as ⟨ b6, hb6 ⟩
  step with (u32_at_spec d b6) as ⟨ o6 ⟩
  rcases o6 with _ | w6
  · elf_done
  try dsimp only
  step as ⟨ b7, hb7 ⟩
  step with (u32_at_spec d b7) as ⟨ o7 ⟩
  rcases o7 with _ | w7
  · elf_done
  try dsimp only
  elf_done

theorem rd_section_total (d : Slice U8) (base : U64) (h : base.val ≤ 8589934592) :
    reader_v.rd_section d base ⦃ _ => True ⦄ := by
  unfold reader_v.rd_section
  step with (u32_at_spec d base) as ⟨ o0 ⟩
  rcases o0 with _ | w0
  · elf_done
  try dsimp only
  step as ⟨ b1, hb1 ⟩
  step with (u32_at_spec d b1) as ⟨ o1 ⟩
  rcases o1 with _ | w1
  · elf_done
  try dsimp only
  step as ⟨ b2, hb2 ⟩
  step with (u32_at_spec d b2) as ⟨ o2 ⟩
  rcases o2 with _ | w2
  · elf_done
  try dsimp only
  step as ⟨ b3, hb3 ⟩
  step with (u32_at_spec d b3) as ⟨ o3 ⟩
  rcases o3 with _ | w3
  · elf_done
  try dsimp only
  step as ⟨ b4, hb4 ⟩
  step with (u32_at_spec d b4) as ⟨ o4 ⟩
  rcases o4 with _ | w4
  · elf_done
  try dsimp only
  step as ⟨ b5, hb5 ⟩
  step with (u32_at_spec d b5) as ⟨ o5 ⟩
  rcases o5 with _ | w5
  · elf_done
  try dsimp only
  step as ⟨ b6, hb6 ⟩
  step with (u32_at_spec d b6) as ⟨ o6 ⟩
  rcases o6 with _ | w6
  · elf_done
  try dsimp only
  step as ⟨ b7, hb7 ⟩
  step with (u32_at_spec d b7) as ⟨ o7 ⟩
  rcases o7 with _ | w7
  · elf_done
  try dsimp only
  step as ⟨ b8, hb8 ⟩
  step with (u32_at_spec d b8) as ⟨ o8 ⟩
  rcases o8 with _ | w8
  · elf_done
  try dsimp only
  step as ⟨ b9, hb9 ⟩
  step with (u32_at_spec d b9) as ⟨ o9 ⟩
  rcases o9 with _ | w9
  · elf_done
  try dsimp only
  elf_done

theorem rd_segments_spec (d : Slice U8) (off : U32) (e n : U16) :
    reader_v.rd_segments d off e n ⦃ r =>
      match r with
      | .Ok v => v.val.length = n.val
      | .Err _ => True ⦄ := by
  unfold reader_v.rd_segments reader_v.rd_segments_loop
  apply loop.spec_decr_nat (fun (s : alloc.vec.Vec reader_v.Segment × U64) => n.val - s.2.val)
    (fun s => s.2.val ≤ n.val ∧ s.1.val.length = s.2.val)
  · intro s hs
    obtain ⟨out, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold reader_v.rd_segments_loop.body
    step as ⟨ i1, hi1 ⟩
    have hi1v : i1.val = n.val := by rw [hi1]; scalar_tac
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      step as ⟨ i2, hi2 ⟩
      have hi2v : i2.val = off.val := by rw [hi2]; scalar_tac
      step as ⟨ i3, hi3 ⟩
      have hi3v : i3.val = e.val := by rw [hi3]; scalar_tac
      have hmul : i.val * i3.val ≤ 65535 * 65535 :=
        Nat.mul_le_mul (by scalar_tac) (by scalar_tac)
      step as ⟨ i4, hi4 ⟩
      step as ⟨ base, hbase ⟩
      step with (rd_segment_total d base (by scalar_tac)) as ⟨ r ⟩
      rcases r with v | x
      · try dsimp only
        have hroom : out.val.length < Usize.max := by scalar_tac
        step as ⟨ out1, hout1 ⟩
        step as ⟨ i5, hi5 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hout1, List.length_append, h2, hi5]
        rfl
      · elf_done
    · rename_i hge
      have hin : i.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      omega
  · exact ⟨by scalar_tac, by simp [alloc.vec.Vec.new]⟩

theorem rd_sections_spec (d : Slice U8) (off : U32) (e n : U16) :
    reader_v.rd_sections d off e n ⦃ r =>
      match r with
      | .Ok v => v.val.length = n.val
      | .Err _ => True ⦄ := by
  unfold reader_v.rd_sections reader_v.rd_sections_loop
  apply loop.spec_decr_nat (fun (s : alloc.vec.Vec reader_v.SecHdr × U64) => n.val - s.2.val)
    (fun s => s.2.val ≤ n.val ∧ s.1.val.length = s.2.val)
  · intro s hs
    obtain ⟨out, i⟩ := s
    obtain ⟨h1, h2⟩ := hs
    dsimp only at h1 h2 ⊢
    unfold reader_v.rd_sections_loop.body
    step as ⟨ i1, hi1 ⟩
    have hi1v : i1.val = n.val := by rw [hi1]; scalar_tac
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      step as ⟨ i2, hi2 ⟩
      have hi2v : i2.val = off.val := by rw [hi2]; scalar_tac
      step as ⟨ i3, hi3 ⟩
      have hi3v : i3.val = e.val := by rw [hi3]; scalar_tac
      have hmul : i.val * i3.val ≤ 65535 * 65535 :=
        Nat.mul_le_mul (by scalar_tac) (by scalar_tac)
      step as ⟨ i4, hi4 ⟩
      step as ⟨ base, hbase ⟩
      step with (rd_section_total d base (by scalar_tac)) as ⟨ r ⟩
      rcases r with v | x
      · try dsimp only
        have hroom : out.val.length < Usize.max := by scalar_tac
        step as ⟨ out1, hout1 ⟩
        step as ⟨ i5, hi5 ⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hout1, List.length_append, h2, hi5]
        rfl
      · elf_done
    · rename_i hge
      have hin : i.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      omega
  · exact ⟨by scalar_tac, by simp [alloc.vec.Vec.new]⟩

theorem rd_names_spec (d : Slice U8) (secs : alloc.vec.Vec reader_v.SecHdr) (base : U32) :
    reader_v.rd_names d secs base ⦃ r =>
      match r with
      | .Ok v => v.val.length = secs.val.length ∧
          ∀ x ∈ v.val, x.1.val ≤ x.2.val ∧ x.2.val ≤ d.val.length
      | .Err _ => True ⦄ := by
  unfold reader_v.rd_names reader_v.rd_names_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec (Usize × Usize) × Usize) => secs.val.length - s.2.val)
    (fun s => s.2.val ≤ secs.val.length ∧ s.1.val.length = s.2.val ∧
      ∀ x ∈ s.1.val, x.1.val ≤ x.2.val ∧ x.2.val ≤ d.val.length)
  · intro s hs
    obtain ⟨out, i⟩ := s
    obtain ⟨h1, h2, h3⟩ := hs
    dsimp only at h1 h2 h3 ⊢
    unfold reader_v.rd_names_loop.body
    try dsimp only
    split
    · rename_i hlt
      have hlt' : i.val < secs.val.length := by scalar_tac
      step as ⟨ i2, hi2 ⟩
      have hi2v : i2.val = base.val := by rw [hi2]; scalar_tac
      step as ⟨ sh, hsh ⟩
      step as ⟨ i3, hi3 ⟩
      have hi3v : i3.val = sh.name.val := by rw [hi3]; scalar_tac
      step as ⟨ off, hoff ⟩
      step as ⟨ i5, hi5 ⟩
      have hi5v : i5.val = d.val.length := by rw [hi5, cast_usize_u64, slice_len_val]
      split
      · elf_done
      rename_i hin
      have hin' : off.val ≤ d.val.length := by scalar_tac
      step as ⟨ st, hst ⟩
      have hstv : st.val = off.val := by
        rw [hst]
        exact cast_u64_usize off d.len (by rw [slice_len_val]; omega)
      step with (str_end_spec d st d.len (by rw [slice_len_val]; omega)
        (by rw [slice_len_val])) as ⟨ e, he ⟩
      obtain ⟨a1, a2, _, _, _⟩ := he
      rw [slice_len_val] at a2
      have hroom : out.val.length < Usize.max := by scalar_tac
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i7, hi7 ⟩
      refine ⟨by omega, ?_, ?_, by omega⟩
      · rw [hout1, List.length_append, h2, hi7]
        rfl
      · intro x hx
        rw [hout1] at hx
        rcases List.mem_append.1 hx with hx | hx
        · exact h3 x hx
        · have hxv := List.mem_singleton.1 hx
          rw [hxv]
          exact ⟨a1, a2⟩
    · rename_i hge
      have hin : i.val = secs.val.length := by scalar_tac
      simp only [WP.spec_ok]
      exact ⟨by omega, h3⟩
  · exact ⟨by scalar_tac, by simp [alloc.vec.Vec.new],
      fun x hx => absurd hx (by simp [alloc.vec.Vec.new])⟩

theorem no_names_spec (n : Usize) :
    reader_v.no_names n ⦃ r => r.val.length = n.val ∧ ∀ x ∈ r.val, x = (0#usize, 0#usize) ⦄ := by
  unfold reader_v.no_names reader_v.no_names_loop
  apply loop.spec_decr_nat
    (fun (s : alloc.vec.Vec (Usize × Usize) × Usize) => n.val - s.2.val)
    (fun s => s.2.val ≤ n.val ∧ s.1.val.length = s.2.val ∧
      ∀ x ∈ s.1.val, x = (0#usize, 0#usize))
  · intro s hs
    obtain ⟨out, i⟩ := s
    obtain ⟨h1, h2, h3⟩ := hs
    dsimp only at h1 h2 h3 ⊢
    unfold reader_v.no_names_loop.body
    split
    · rename_i hlt
      have hlt' : i.val < n.val := by scalar_tac
      have hroom : out.val.length < Usize.max := by scalar_tac
      step as ⟨ out1, hout1 ⟩
      step as ⟨ i1, hi1 ⟩
      refine ⟨by omega, ?_, ?_, by omega⟩
      · rw [hout1, List.length_append, h2, hi1]
        rfl
      · intro x hx
        rw [hout1] at hx
        rcases List.mem_append.1 hx with hx | hx
        · exact h3 x hx
        · exact List.mem_singleton.1 hx
    · rename_i hge
      have hin : i.val = n.val := by scalar_tac
      simp only [WP.spec_ok]
      exact ⟨by omega, h3⟩
  · exact ⟨by scalar_tac, by simp [alloc.vec.Vec.new],
      fun x hx => absurd hx (by simp [alloc.vec.Vec.new])⟩

/-- The header's fields are the words at the System V ABI's offsets
(`reader_v::header_of`). -/
def HeaderOk (d : List U8) (h : reader_v.Header) : Prop :=
  h.kind.bv = le16 d 16 ∧ h.machine.bv = le16 d 18 ∧ h.entry.bv = le32 d 24 ∧
  h.phoff.bv = le32 d 28 ∧ h.shoff.bv = le32 d 32 ∧ h.flags.bv = le32 d 36 ∧
  h.phentsize.bv = le16 d 42 ∧ h.phnum.bv = le16 d 44 ∧ h.shentsize.bv = le16 d 46 ∧
  h.shnum.bv = le16 d 48 ∧ h.shstrndx.bv = le16 d 50

/-- **`parse`** returns for every byte string: no read is out of range
and no offset overflows. When it returns a parse, the file has at least
the 52 bytes of an ELF32 header, the header's fields are the words at
their offsets, there is one program header per `phnum` and one section
header and one name extent per `shnum`, and every name extent lies in the
file. (That each program and section header is the words at its offset is
proven in Verus, `parsed_ok`, and not restated here.) -/
theorem parse_spec (d : Slice U8) :
    reader_v.parse d ⦃ r =>
      match r with
      | .Ok p => 52 ≤ d.val.length ∧ HeaderOk d.val p.header ∧
          p.segments.val.length = p.header.phnum.val ∧
          p.sections.val.length = p.header.shnum.val ∧
          p.names.val.length = p.sections.val.length ∧
          ∀ x ∈ p.names.val, x.1.val ≤ x.2.val ∧ x.2.val ≤ d.val.length
      | .Err _ => True ⦄ := by
  unfold reader_v.parse
  try dsimp only
  split
  · elf_done
  rename_i h52
  have h52' : 52 ≤ d.val.length := by scalar_tac
  have h52'' : 52 ≤ d.length := by scalar_tac
  step as ⟨ i1, hi1 ⟩
  split
  · elf_done
  step as ⟨ i2, hi2 ⟩
  split
  · elf_done
  step as ⟨ i3, hi3 ⟩
  split
  · elf_done
  step as ⟨ i4, hi4 ⟩
  split
  · elf_done
  step as ⟨ i5, hi5 ⟩
  split
  · elf_done
  step as ⟨ i6, hi6 ⟩
  split
  · elf_done
  step with (u16_at_spec d 16#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_kind⟩ := ho (by scalar_tac)
  have hww : v = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u16_at_spec d 18#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v1
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_machine⟩ := ho (by scalar_tac)
  have hww : v1 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u32_at_spec d 24#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v2
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_entry⟩ := ho (by scalar_tac)
  have hww : v2 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u32_at_spec d 28#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v3
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_phoff⟩ := ho (by scalar_tac)
  have hww : v3 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u32_at_spec d 32#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v4
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_shoff⟩ := ho (by scalar_tac)
  have hww : v4 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u32_at_spec d 36#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v5
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_flags⟩ := ho (by scalar_tac)
  have hww : v5 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u16_at_spec d 42#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v6
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_phentsize⟩ := ho (by scalar_tac)
  have hww : v6 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u16_at_spec d 44#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v7
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_phnum⟩ := ho (by scalar_tac)
  have hww : v7 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u16_at_spec d 46#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v8
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_shentsize⟩ := ho (by scalar_tac)
  have hww : v8 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u16_at_spec d 48#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v9
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_shnum⟩ := ho (by scalar_tac)
  have hww : v9 = w := Option.some.inj hw
  subst hww
  clear hw ho
  step with (u16_at_spec d 50#u64) as ⟨ o, ho, _ ⟩
  rcases o with _ | v10
  · elf_done
  try dsimp only
  obtain ⟨w, hw, hb_shstrndx⟩ := ho (by scalar_tac)
  have hww : v10 = w := Option.some.inj hw
  subst hww
  clear hw ho
  have hall : v.bv = le16 d.val 16 ∧ v1.bv = le16 d.val 18 ∧ v2.bv = le32 d.val 24 ∧
      v3.bv = le32 d.val 28 ∧ v4.bv = le32 d.val 32 ∧ v5.bv = le32 d.val 36 ∧
      v6.bv = le16 d.val 42 ∧ v7.bv = le16 d.val 44 ∧ v8.bv = le16 d.val 46 ∧
      v9.bv = le16 d.val 48 ∧ v10.bv = le16 d.val 50 :=
    ⟨hb_kind, hb_machine, hb_entry, hb_phoff, hb_shoff, hb_flags, hb_phentsize, hb_phnum,
      hb_shentsize, hb_shnum, hb_shstrndx⟩
  step with (rd_segments_spec d v3 v6 v7) as ⟨ r, hr ⟩
  rcases r with v11 | x
  swap
  · elf_done
  dsimp only at hr ⊢
  step with (rd_sections_spec d v4 v8 v9) as ⟨ r1, hr1 ⟩
  rcases r1 with v12 | x
  swap
  · elf_done
  dsimp only at hr1 ⊢
  step as ⟨ i7, hi7 ⟩
  split
  · rename_i hlt
    step as ⟨ i9, hi9 ⟩
    have hb9 : i9.val < v12.val.length := by
      have e1 : i9 = i7 := by rw [hi9, hi7]
      rw [e1]; scalar_tac
    step as ⟨ sh, hsh ⟩
    step with (rd_names_spec d v12 sh.offset) as ⟨ r2, hr2 ⟩
    rcases r2 with v13 | x
    swap
    · elf_done
    dsimp only at hr2 ⊢
    rw [WP.spec_ok]
    refine ⟨h52', ?_, hr, hr1, hr2.1, hr2.2⟩
    unfold HeaderOk
    exact hall
  · rename_i hge
    step with (no_names_spec v12.len) as ⟨ names, hn1, hn2 ⟩
    refine ⟨h52', ?_, hr, hr1, by rw [hn1]; scalar_tac, ?_⟩
    · unfold HeaderOk
      exact hall
    · intro x hx
      rw [hn2 x hx]
      constructor <;> simp

/-! ## Axioms -/

#print axioms u16_at_spec
#print axioms u32_at_spec
#print axioms str_end_spec
#print axioms section_range_spec
#print axioms sym_entry_spec
#print axioms sym_entries_spec
#print axioms symbols_spec
#print axioms name_is_spec
#print axioms find_spec
#print axioms lookup_spec
#print axioms section_at_spec
#print axioms copy_range_spec
#print axioms zeros_spec
#print axioms read_at_spec
#print axioms rd_segments_spec
#print axioms rd_sections_spec
#print axioms rd_names_spec
#print axioms no_names_spec
#print axioms parse_spec

end rsk_elf
