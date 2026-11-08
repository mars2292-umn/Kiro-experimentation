//! Communication deadlock freedom (R41.3), mechanized in Verus (task 21.6,
//! 2026-11-08).
//!
//! # The model
//!
//! A Protocol-governed Endpoint is a finite set of roles, each a finite
//! automaton over the states of its projection (`Trans`: in state `from`
//! the role sends `label` to `peer`, or receives `label` from `peer`, and
//! moves to `to`; the Generator's typestate API offers exactly these
//! operations in each state, R38.3, R38.4), and one FIFO channel per
//! ordered pair of roles (`Config::chans[from * n + to]`, the Kernel's
//! buffer of task 21.2). A configuration is every role's state and every
//! channel's content. A send is enabled whenever the role's state allows it
//! (the Kernel refuses a send into a full channel as a fault, R40.5, so the
//! model lets every send through and the certificate shows no channel ever
//! holds more than the capacity, which is R41.1); a receive is enabled when
//! the channel's oldest message carries the expected label (`enabled`,
//! `succ`, `step`). A run starts with every role in its initial state and
//! every channel empty (`initial`) and takes steps (`run`, `reachable`).
//!
//! # The property
//!
//! R41.3's deadlock is a non-empty set of roles each of which awaits a
//! message that only another role of the set can send: `deadlock_set`
//! says some role of the set is `blocked` (its state has operations, each
//! a receive, none enabled, so the role awaits a message, within its
//! Protocol or as its next Release), every role of the set is blocked or
//! `terminal` (finished: it will never send again), and the sender of each
//! awaited channel is in the set (every channel has one sender, the role
//! at the other end). Counting finished senders makes a wait on a peer
//! that has ended a deadlock too (strengthened 2026-11-08).
//! `deadlocked` is the existence of such a set. The theorem
//! (`theorem_r41_3`) is that no reachable configuration is deadlocked, and
//! that every reachable channel content fits the declared capacity.
//!
//! # The method
//!
//! A certificate (`certificate`) is a list of configurations that contains
//! the initial one, is closed under every enabled step, and has no
//! deadlocked configuration and no channel over the capacity. By induction
//! on the number of steps every reachable configuration is in the list
//! (`lemma_run_in_list`), which gives the theorem. The untrusted
//! explorer of `explore.rs` produces the list (its visited set); the
//! verified `certify` checks it against the specification above and
//! returns `true` only when the certificate holds, so an accepted Protocol
//! carries the proof for its own interleavings. `has_deadlock` decides
//! `deadlocked` by the greatest fixed point of the blocked roles under
//! "every awaited sender is in the set": every deadlock set is a
//! post-fixed point, so it lies inside the result, and a non-empty result
//! is itself a deadlock set.
//!
//! The assumptions that connect the model to an Application are listed in
//! the Verification_Plan (5.15).

use vstd::prelude::*;

verus! {

/// One operation of a role's projection, as the typestate API offers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trans {
    /// The role that performs the operation.
    pub role: usize,
    /// The role's state before it.
    pub from: usize,
    /// A send (`true`) to `peer`, or a receive (`false`) from `peer`.
    pub send: bool,
    pub peer: usize,
    pub label: u32,
    /// The role's state after it.
    pub to: usize,
}

/// The roles' automata of one Endpoint and the declared capacity of each
/// directed channel.
pub struct System {
    pub roles: usize,
    pub capacity: usize,
    pub trans: Vec<Trans>,
}

/// A configuration: every role's state, and every directed channel's
/// labels in transit, oldest first (`chans[from * roles + to]`).
pub struct Config {
    pub states: Vec<usize>,
    pub chans: Vec<Vec<u32>>,
}

/// The configuration as the specification sees it.
pub ghost struct Cfg {
    pub states: Seq<usize>,
    pub chans: Seq<Seq<u32>>,
}

pub open spec fn cfg(c: Config) -> Cfg {
    Cfg { states: c.states@, chans: c.chans@.map(|i: int, v: Vec<u32>| v@) }
}

pub open spec fn cfgs(list: Vec<Config>) -> Seq<Cfg> {
    list@.map(|i: int, c: Config| cfg(c))
}

/// At most 16 roles (the channel index `from * roles + to` stays small),
/// every operation between two distinct roles of the Endpoint.
pub open spec fn wf_sys(s: System) -> bool {
    &&& s.roles <= 16
    &&& forall|i: int| 0 <= i < s.trans@.len() ==> {
        let t = #[trigger] s.trans@[i];
        t.role < s.roles && t.peer < s.roles && t.peer != t.role
    }
}

pub open spec fn wf_cfg(n: int, c: Cfg) -> bool {
    c.states.len() == n && c.chans.len() == n * n
}

/// The channel an operation uses: from the sender to the receiver.
pub open spec fn chan_of(n: int, t: Trans) -> int {
    if t.send { (t.role as int) * n + (t.peer as int) } else { (t.peer as int) * n + (t.role as int) }
}

/// The role is in the operation's state.
pub open spec fn at(c: Cfg, t: Trans) -> bool {
    t.role < c.states.len() && c.states[t.role as int] == t.from
}

/// A send is enabled in its state; a receive when the oldest message of
/// its channel carries its label.
pub open spec fn enabled(n: int, c: Cfg, t: Trans) -> bool {
    &&& at(c, t)
    &&& 0 <= chan_of(n, t) < c.chans.len()
    &&& (t.send || (c.chans[chan_of(n, t)].len() > 0 && c.chans[chan_of(n, t)][0] == t.label))
}

/// The configuration after an enabled operation.
pub open spec fn succ(n: int, c: Cfg, t: Trans) -> Cfg {
    let ch = chan_of(n, t);
    Cfg {
        states: c.states.update(t.role as int, t.to),
        chans: c.chans.update(ch, if t.send { c.chans[ch].push(t.label) } else { c.chans[ch].drop_first() }),
    }
}

pub open spec fn step(s: System, a: Cfg, b: Cfg) -> bool {
    exists|i: int| 0 <= i < s.trans@.len() && enabled(s.roles as int, a, #[trigger] s.trans@[i]) && b == succ(s.roles as int, a, s.trans@[i])
}

/// Every role in its initial state (0: the Generator's `S0`, the zeroed
/// image of a Partition restart), every channel empty.
pub open spec fn initial(n: int) -> Cfg {
    Cfg { states: Seq::new(n as nat, |i: int| 0usize), chans: Seq::new((n * n) as nat, |i: int| Seq::<u32>::empty()) }
}

/// A run: the initial configuration, then a step at a time.
pub open spec fn run(s: System, path: Seq<Cfg>) -> bool {
    &&& path.len() > 0
    &&& path[0] == initial(s.roles as int)
    &&& forall|i: int| 1 <= i < path.len() ==> step(s, path[i - 1], #[trigger] path[i])
}

pub open spec fn reachable(s: System, c: Cfg) -> bool {
    exists|path: Seq<Cfg>| #[trigger] run(s, path) && path.last() == c
}

/// The role has an operation in its state.
pub open spec fn has_trans(s: System, c: Cfg, r: int) -> bool {
    exists|i: int| 0 <= i < s.trans@.len() && (#[trigger] s.trans@[i]).role == r && at(c, s.trans@[i])
}

/// The role awaits a message: it has operations, each a receive, none
/// enabled.
pub open spec fn blocked(s: System, c: Cfg, r: int) -> bool {
    &&& has_trans(s, c, r)
    &&& forall|i: int| 0 <= i < s.trans@.len() && (#[trigger] s.trans@[i]).role == r && at(c, s.trans@[i])
        ==> !s.trans@[i].send && !enabled(s.roles as int, c, s.trans@[i])
}

/// The role has finished: no operation in its state (an `end`). It will
/// never send again, so a role awaiting it waits for ever.
pub open spec fn terminal(s: System, c: Cfg, r: int) -> bool {
    !has_trans(s, c, r)
}

/// R41.3's waiting set, strengthened (2026-11-08): at least one role of
/// the set awaits a message, every role of the set awaits or has finished,
/// and the sender of every awaited channel is in the set (so it awaits
/// too, or has finished and will never send).
pub open spec fn deadlock_set(s: System, c: Cfg, set: Seq<bool>) -> bool {
    &&& set.len() == s.roles
    &&& exists|r: int| 0 <= r < set.len() && set[r] && blocked(s, c, r)
    &&& forall|r: int| 0 <= r < set.len() && #[trigger] set[r] ==> blocked(s, c, r) || terminal(s, c, r)
    &&& forall|i: int| 0 <= i < s.trans@.len() && set[(#[trigger] s.trans@[i]).role as int] && at(c, s.trans@[i])
        ==> set[s.trans@[i].peer as int]
}

pub open spec fn deadlocked(s: System, c: Cfg) -> bool {
    exists|set: Seq<bool>| deadlock_set(s, c, set)
}

/// No channel holds more than the declared capacity (R41.1).
pub open spec fn within_capacity(s: System, c: Cfg) -> bool {
    forall|ch: int| 0 <= ch < c.chans.len() ==> (#[trigger] c.chans[ch]).len() <= s.capacity
}

pub open spec fn in_list(list: Seq<Cfg>, c: Cfg) -> bool {
    exists|i: int| 0 <= i < list.len() && list[i] == c
}

/// A certificate of R41.3 and R41.1 for `s`: the initial configuration is
/// listed, every listed configuration is well formed, within the capacity,
/// and not deadlocked, and every enabled operation of a listed
/// configuration leads to a listed one.
pub open spec fn certificate(s: System, list: Seq<Cfg>) -> bool {
    &&& in_list(list, initial(s.roles as int))
    &&& forall|i: int| 0 <= i < list.len()
        ==> wf_cfg(s.roles as int, #[trigger] list[i]) && within_capacity(s, list[i]) && !deadlocked(s, list[i])
    &&& forall|i: int, j: int| 0 <= i < list.len() && 0 <= j < s.trans@.len() && enabled(s.roles as int, #[trigger] list[i], #[trigger] s.trans@[j])
        ==> in_list(list, succ(s.roles as int, list[i], s.trans@[j]))
}

/// Every configuration a run reaches is in a certificate's list.
pub proof fn lemma_run_in_list(s: System, list: Seq<Cfg>, path: Seq<Cfg>)
    requires
        certificate(s, list),
        run(s, path),
    ensures
        in_list(list, path.last()),
    decreases path.len(),
{
    if path.len() == 1 {
        assert(path.last() == initial(s.roles as int));
    } else {
        let prefix = path.drop_last();
        assert(run(s, prefix));
        lemma_run_in_list(s, list, prefix);
        let p = prefix.last();
        let c = path.last();
        assert(step(s, p, c));
        let i = choose|i: int| 0 <= i < list.len() && list[i] == p;
        let j = choose|j: int| 0 <= j < s.trans@.len() && enabled(s.roles as int, p, #[trigger] s.trans@[j]) && c == succ(s.roles as int, p, s.trans@[j]);
        assert(enabled(s.roles as int, list[i], s.trans@[j]));
        assert(in_list(list, succ(s.roles as int, list[i], s.trans@[j])));
    }
}

/// R41.3 (and R41.1) for every run of an Endpoint whose explorer's list is
/// a certificate: no reachable configuration has a set of roles each
/// awaiting a message that only another role of the set can send, and no
/// reachable channel content exceeds the capacity.
pub proof fn theorem_r41_3(s: System, list: Seq<Cfg>)
    requires
        certificate(s, list),
    ensures
        forall|c: Cfg| reachable(s, c) ==> !deadlocked(s, c) && within_capacity(s, c),
{
    assert forall|c: Cfg| reachable(s, c) implies !deadlocked(s, c) && within_capacity(s, c) by {
        let path = choose|path: Seq<Cfg>| #[trigger] run(s, path) && path.last() == c;
        lemma_run_in_list(s, list, path);
    }
}

pub proof fn lemma_chan_in_range(n: int, a: int, b: int)
    requires
        0 <= a < n,
        0 <= b < n,
    ensures
        0 <= a * n + b < n * n,
{
    assert(0 <= a * n + b < n * n) by (nonlinear_arith)
        requires 0 <= a < n, 0 <= b < n;
}

proof fn lemma_small_product(a: int, b: int)
    requires
        0 <= a <= 16,
        0 <= b <= 16,
    ensures
        a * b <= 256,
{
    assert(a * b <= 256) by (nonlinear_arith)
        requires 0 <= a <= 16, 0 <= b <= 16;
}

/// The members of a set of roles (the measure of `has_deadlock`'s fixed
/// point).
pub open spec fn count_true(s: Seq<bool>) -> nat
    decreases s.len(),
{
    if s.len() == 0 { 0 } else { count_true(s.drop_last()) + (if s.last() { 1nat } else { 0nat }) }
}

proof fn lemma_count_push(s: Seq<bool>, b: bool)
    ensures
        count_true(s.push(b)) == count_true(s) + (if b { 1nat } else { 0nat }),
{
    assert(s.push(b).drop_last() =~= s);
}

proof fn lemma_count_remove(s: Seq<bool>, i: int)
    requires
        0 <= i < s.len(),
        s[i],
    ensures
        count_true(s.update(i, false)) + 1 == count_true(s),
    decreases s.len(),
{
    if i == s.len() - 1 {
        assert(s.update(i, false).drop_last() =~= s.drop_last());
    } else {
        assert(s.update(i, false).drop_last() =~= s.drop_last().update(i, false));
        lemma_count_remove(s.drop_last(), i);
    }
}

// ---------------------------------------------------------------- exec --

fn copy_from(src: &Vec<u32>, start: usize) -> (r: Vec<u32>)
    requires
        start <= src.len(),
    ensures
        r@ == src@.subrange(start as int, src@.len() as int),
{
    let mut out: Vec<u32> = Vec::new();
    let mut i = start;
    while i < src.len()
        invariant
            start <= i <= src.len(),
            out@ == src@.subrange(start as int, i as int),
        decreases src.len() - i,
    {
        out.push(src[i]);
        i += 1;
        assert(out@ =~= src@.subrange(start as int, i as int));
    }
    out
}

fn eq_u32(a: &Vec<u32>, b: &Vec<u32>) -> (r: bool)
    ensures
        r == (a@ == b@),
{
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len()
        invariant
            i <= a.len(),
            a.len() == b.len(),
            forall|j: int| 0 <= j < i ==> a@[j] == b@[j],
        decreases a.len() - i,
    {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    assert(a@ =~= b@);
    true
}

fn eq_usize(a: &Vec<usize>, b: &Vec<usize>) -> (r: bool)
    ensures
        r == (a@ == b@),
{
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len()
        invariant
            i <= a.len(),
            a.len() == b.len(),
            forall|j: int| 0 <= j < i ==> a@[j] == b@[j],
        decreases a.len() - i,
    {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    assert(a@ =~= b@);
    true
}

fn eq_cfg(a: &Config, b: &Config) -> (r: bool)
    ensures
        r == (cfg(*a) == cfg(*b)),
{
    if !eq_usize(&a.states, &b.states) {
        assert(cfg(*a).states != cfg(*b).states);
        return false;
    }
    if a.chans.len() != b.chans.len() {
        assert(cfg(*a).chans.len() != cfg(*b).chans.len());
        return false;
    }
    let mut i = 0;
    while i < a.chans.len()
        invariant
            i <= a.chans.len(),
            a.chans.len() == b.chans.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] a.chans@[j])@ == b.chans@[j]@,
        decreases a.chans.len() - i,
    {
        if !eq_u32(&a.chans[i], &b.chans[i]) {
            assert(cfg(*a).chans[i as int] == a.chans@[i as int]@);
            assert(cfg(*b).chans[i as int] == b.chans@[i as int]@);
            assert(cfg(*a).chans[i as int] != cfg(*b).chans[i as int]);
            return false;
        }
        i += 1;
    }
    assert(cfg(*a).states == cfg(*b).states);
    assert forall|j: int| 0 <= j < cfg(*a).chans.len() implies cfg(*a).chans[j] == cfg(*b).chans[j] by {
        assert(cfg(*a).chans[j] == a.chans@[j]@);
        assert(cfg(*b).chans[j] == b.chans@[j]@);
    }
    assert(cfg(*a).chans =~= cfg(*b).chans);
    true
}

fn contains(list: &Vec<Config>, c: &Config) -> (r: bool)
    ensures
        r == in_list(cfgs(*list), cfg(*c)),
{
    let mut i = 0;
    while i < list.len()
        invariant
            i <= list.len(),
            forall|j: int| 0 <= j < i ==> cfgs(*list)[j] != cfg(*c),
        decreases list.len() - i,
    {
        if eq_cfg(&list[i], c) {
            assert(cfgs(*list)[i as int] == cfg(*c));
            return true;
        }
        i += 1;
    }
    false
}

fn initial_config(n: usize) -> (r: Config)
    requires
        n <= 16,
    ensures
        cfg(r) == initial(n as int),
{
    proof { lemma_small_product(n as int, n as int); }
    let mut states: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < n
        invariant
            i <= n,
            states@ == Seq::new(i as nat, |k: int| 0usize),
        decreases n - i,
    {
        states.push(0);
        i += 1;
        assert(states@ =~= Seq::new(i as nat, |k: int| 0usize));
    }
    let mut chans: Vec<Vec<u32>> = Vec::new();
    let mut j = 0;
    while j < n * n
        invariant
            n <= 16,
            n * n <= 256,
            j <= n * n,
            chans.len() == j,
            forall|k: int| 0 <= k < j ==> (#[trigger] chans@[k])@.len() == 0,
        decreases n * n - j,
    {
        chans.push(Vec::new());
        j += 1;
    }
    let r = Config { states, chans };
    assert forall|k: int| 0 <= k < cfg(r).chans.len() implies cfg(r).chans[k] == initial(n as int).chans[k] by {
        assert(cfg(r).chans[k] == chans@[k]@);
        assert(chans@[k]@ =~= Seq::<u32>::empty());
    }
    assert(cfg(r).chans =~= initial(n as int).chans);
    assert(cfg(r).states =~= initial(n as int).states);
    r
}

fn wf_config(s: &System, c: &Config) -> (r: bool)
    requires
        wf_sys(*s),
    ensures
        r == wf_cfg(s.roles as int, cfg(*c)),
{
    proof { lemma_small_product(s.roles as int, s.roles as int); }
    c.states.len() == s.roles && c.chans.len() == s.roles * s.roles
}

fn capacity_ok(s: &System, c: &Config) -> (r: bool)
    ensures
        r == within_capacity(*s, cfg(*c)),
{
    let mut i = 0;
    while i < c.chans.len()
        invariant
            i <= c.chans.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] cfg(*c).chans[j]).len() <= s.capacity,
        decreases c.chans.len() - i,
    {
        if c.chans[i].len() > s.capacity {
            assert(cfg(*c).chans[i as int].len() > s.capacity);
            return false;
        }
        i += 1;
    }
    true
}

/// The channel index of a transition, in range for a well-formed system.
fn chan_index(s: &System, t: Trans) -> (r: usize)
    requires
        wf_sys(*s),
        t.role < s.roles,
        t.peer < s.roles,
    ensures
        r == chan_of(s.roles as int, t),
        r < s.roles * s.roles,
{
    proof {
        lemma_small_product(s.roles as int, s.roles as int);
        lemma_chan_in_range(s.roles as int, t.role as int, t.peer as int);
        lemma_chan_in_range(s.roles as int, t.peer as int, t.role as int);
    }
    if t.send { t.role * s.roles + t.peer } else { t.peer * s.roles + t.role }
}

fn is_enabled(s: &System, c: &Config, i: usize) -> (r: bool)
    requires
        wf_sys(*s),
        i < s.trans.len(),
        wf_cfg(s.roles as int, cfg(*c)),
    ensures
        r == enabled(s.roles as int, cfg(*c), s.trans@[i as int]),
{
    let t = s.trans[i];
    if t.role >= c.states.len() || c.states[t.role] != t.from {
        return false;
    }
    let ch = chan_index(s, t);
    if t.send {
        return true;
    }
    let q: &Vec<u32> = &c.chans[ch];
    q.len() > 0 && q[0] == t.label
}

fn successor(s: &System, c: &Config, i: usize) -> (r: Config)
    requires
        wf_sys(*s),
        i < s.trans.len(),
        wf_cfg(s.roles as int, cfg(*c)),
        enabled(s.roles as int, cfg(*c), s.trans@[i as int]),
    ensures
        cfg(r) == succ(s.roles as int, cfg(*c), s.trans@[i as int]),
{
    let t = s.trans[i];
    let ch = chan_index(s, t);
    let ghost target = succ(s.roles as int, cfg(*c), t);
    let mut states: Vec<usize> = Vec::new();
    let mut k = 0;
    while k < c.states.len()
        invariant
            k <= c.states.len(),
            t.role < c.states.len(),
            states@ == target.states.subrange(0, k as int),
            target.states == cfg(*c).states.update(t.role as int, t.to),
        decreases c.states.len() - k,
    {
        states.push(if k == t.role { t.to } else { c.states[k] });
        k += 1;
        assert(states@ =~= target.states.subrange(0, k as int));
    }
    assert(states@ =~= target.states);
    let mut chans: Vec<Vec<u32>> = Vec::new();
    let mut m = 0;
    while m < c.chans.len()
        invariant
            m <= c.chans.len(),
            ch < c.chans.len(),
            ch == chan_of(s.roles as int, t),
            enabled(s.roles as int, cfg(*c), t),
            chans.len() == m,
            forall|j: int| 0 <= j < m ==> (#[trigger] chans@[j])@ == target.chans[j],
            target.chans == cfg(*c).chans.update(ch as int, if t.send { cfg(*c).chans[ch as int].push(t.label) } else { cfg(*c).chans[ch as int].drop_first() }),
        decreases c.chans.len() - m,
    {
        if m == ch {
            if t.send {
                let mut q = copy_from(&c.chans[m], 0);
                assert(q@ =~= cfg(*c).chans[ch as int]);
                q.push(t.label);
                chans.push(q);
            } else {
                let q = copy_from(&c.chans[m], 1);
                assert(q@ =~= cfg(*c).chans[ch as int].drop_first());
                chans.push(q);
            }
        } else {
            let q = copy_from(&c.chans[m], 0);
            assert(q@ =~= cfg(*c).chans[m as int]);
            chans.push(q);
        }
        m += 1;
    }
    let r = Config { states, chans };
    assert forall|j: int| 0 <= j < cfg(r).chans.len() implies #[trigger] cfg(r).chans[j] == target.chans[j] by {
        assert(cfg(r).chans[j] == chans@[j]@);
    }
    assert(cfg(r).chans =~= target.chans);
    assert(cfg(r).states == target.states);
    r
}

fn is_blocked(s: &System, c: &Config, r: usize) -> (b: bool)
    requires
        wf_sys(*s),
        wf_cfg(s.roles as int, cfg(*c)),
    ensures
        b == blocked(*s, cfg(*c), r as int),
{
    let mut any = false;
    let mut waits = true;
    let mut i = 0;
    while i < s.trans.len()
        invariant
            i <= s.trans.len(),
            wf_sys(*s),
            wf_cfg(s.roles as int, cfg(*c)),
            any == exists|j: int| 0 <= j < i && (#[trigger] s.trans@[j]).role == r && at(cfg(*c), s.trans@[j]),
            waits == forall|j: int| 0 <= j < i && (#[trigger] s.trans@[j]).role == r && at(cfg(*c), s.trans@[j])
                ==> !s.trans@[j].send && !enabled(s.roles as int, cfg(*c), s.trans@[j]),
        decreases s.trans.len() - i,
    {
        let t = s.trans[i];
        if t.role == r && t.role < c.states.len() && c.states[t.role] == t.from {
            any = true;
            if t.send || is_enabled(s, c, i) {
                waits = false;
            }
        }
        i += 1;
    }
    any && waits
}

fn is_terminal(s: &System, c: &Config, r: usize) -> (b: bool)
    requires
        wf_sys(*s),
        wf_cfg(s.roles as int, cfg(*c)),
    ensures
        b == terminal(*s, cfg(*c), r as int),
{
    let mut any = false;
    let mut i = 0;
    while i < s.trans.len()
        invariant
            i <= s.trans.len(),
            wf_cfg(s.roles as int, cfg(*c)),
            any == exists|j: int| 0 <= j < i && (#[trigger] s.trans@[j]).role == r && at(cfg(*c), s.trans@[j]),
        decreases s.trans.len() - i,
    {
        let t = s.trans[i];
        if t.role == r && t.role < c.states.len() && c.states[t.role] == t.from {
            any = true;
        }
        i += 1;
    }
    !any
}

/// Decides `deadlocked` by the greatest fixed point: start from the
/// blocked and finished roles and remove any role that awaits a sender
/// outside the set until none is left to remove; the result is a waiting
/// set when it has a blocked member.
fn has_deadlock(s: &System, c: &Config) -> (b: bool)
    requires
        wf_sys(*s),
        wf_cfg(s.roles as int, cfg(*c)),
    ensures
        b == deadlocked(*s, cfg(*c)),
{
    let ghost v = cfg(*c);
    let mut cur: Vec<bool> = Vec::new();
    let ghost mut remaining: nat = 0;
    let mut r = 0;
    while r < s.roles
        invariant
            r <= s.roles,
            wf_sys(*s),
            wf_cfg(s.roles as int, v),
            v == cfg(*c),
            cur.len() == r,
            remaining == count_true(cur@),
            remaining <= r,
            forall|q: int| 0 <= q < r ==> cur@[q] == (blocked(*s, v, q) || terminal(*s, v, q)),
        decreases s.roles - r,
    {
        let b = is_blocked(s, c, r) || is_terminal(s, c, r);
        proof { lemma_count_push(cur@, b); }
        cur.push(b);
        proof {
            if b {
                remaining = remaining + 1;
            }
        }
        r += 1;
    }
    let mut changed = true;
    while changed
        invariant
            wf_sys(*s),
            wf_cfg(s.roles as int, v),
            v == cfg(*c),
            cur.len() == s.roles,
            remaining == count_true(cur@),
            forall|q: int| 0 <= q < s.roles && cur@[q] ==> blocked(*s, v, q) || terminal(*s, v, q),
            forall|set: Seq<bool>, q: int| deadlock_set(*s, v, set) && 0 <= q < s.roles && set[q] ==> cur@[q],
            !changed ==> forall|i: int| 0 <= i < s.trans@.len() && cur@[(#[trigger] s.trans@[i]).role as int] && at(v, s.trans@[i])
                ==> cur@[s.trans@[i].peer as int],
        decreases remaining + (if changed { 1int } else { 0int }),
    {
        // A round that changes removes a member of the set.
        changed = false;
        let ghost old_remaining = remaining;
        let mut q = 0;
        while q < s.roles
            invariant
                q <= s.roles,
                wf_sys(*s),
                wf_cfg(s.roles as int, v),
                v == cfg(*c),
                cur.len() == s.roles,
                remaining == count_true(cur@),
                changed ==> remaining < old_remaining,
                remaining <= old_remaining,
                forall|x: int| 0 <= x < s.roles && cur@[x] ==> blocked(*s, v, x) || terminal(*s, v, x),
                forall|set: Seq<bool>, x: int| deadlock_set(*s, v, set) && 0 <= x < s.roles && set[x] ==> cur@[x],
                !changed ==> forall|i: int| 0 <= i < s.trans@.len() && (#[trigger] s.trans@[i]).role < q && cur@[s.trans@[i].role as int] && at(v, s.trans@[i])
                    ==> cur@[s.trans@[i].peer as int],
            decreases s.roles - q,
        {
            if cur[q] {
                let mut i = 0;
                let mut removed = false;
                while i < s.trans.len()
                    invariant
                        i <= s.trans.len(),
                        q < s.roles,
                        wf_sys(*s),
                        wf_cfg(s.roles as int, v),
                        v == cfg(*c),
                        cur.len() == s.roles,
                        remaining == count_true(cur@),
                        changed ==> remaining < old_remaining,
                        remaining <= old_remaining,
                        forall|x: int| 0 <= x < s.roles && cur@[x] ==> blocked(*s, v, x) || terminal(*s, v, x),
                        forall|set: Seq<bool>, x: int| deadlock_set(*s, v, set) && 0 <= x < s.roles && set[x] ==> cur@[x],
                        !changed ==> forall|j: int| 0 <= j < s.trans@.len() && (#[trigger] s.trans@[j]).role < q && cur@[s.trans@[j].role as int] && at(v, s.trans@[j])
                            ==> cur@[s.trans@[j].peer as int],
                        !removed ==> cur@[q as int],
                        !removed ==> forall|j: int| 0 <= j < i && (#[trigger] s.trans@[j]).role == q && at(v, s.trans@[j])
                            ==> cur@[s.trans@[j].peer as int],
                        removed ==> changed,
                    decreases s.trans.len() - i,
                {
                    let t = s.trans[i];
                    if !removed && t.role == q && t.role < c.states.len() && c.states[t.role] == t.from && !cur[t.peer] {
                        // `q` awaits a sender outside the set: no deadlock
                        // set contains `q`, since it would contain the sender.
                        assert(t == s.trans@[i as int]);
                        assert(at(v, t));
                        assert forall|set: Seq<bool>| #[trigger] deadlock_set(*s, v, set) implies !set[q as int] by {
                            if set[q as int] {
                                assert(set[t.peer as int]);
                                assert(cur@[t.peer as int]);
                            }
                        }
                        proof {
                            lemma_count_remove(cur@, q as int);
                            remaining = (remaining - 1) as nat;
                        }
                        cur.set(q, false);
                        removed = true;
                        changed = true;
                    }
                    i += 1;
                }
            }
            q += 1;
        }
    }
    // The fixed point: a result with a blocked member is a deadlock set,
    // and one without shows no deadlock set exists.
    let mut any = false;
    let mut x = 0;
    while x < s.roles
        invariant
            x <= s.roles,
            wf_sys(*s),
            wf_cfg(s.roles as int, v),
            v == cfg(*c),
            cur.len() == s.roles,
            any == exists|y: int| 0 <= y < x && cur@[y] && blocked(*s, v, y),
        decreases s.roles - x,
    {
        if cur[x] && is_blocked(s, c, x) {
            any = true;
        }
        x += 1;
    }
    proof {
        if any {
            assert(cur@.len() == s.roles);
            assert(exists|r: int| 0 <= r < cur@.len() && cur@[r] && blocked(*s, v, r));
            assert(forall|r: int| 0 <= r < cur@.len() && cur@[r] ==> blocked(*s, v, r) || terminal(*s, v, r));
            assert(forall|i: int| 0 <= i < s.trans@.len() && cur@[(#[trigger] s.trans@[i]).role as int] && at(v, s.trans@[i])
                ==> cur@[s.trans@[i].peer as int]);
            assert(deadlock_set(*s, v, cur@));
        } else {
            if deadlocked(*s, v) {
                let set = choose|set: Seq<bool>| deadlock_set(*s, v, set);
                let y = choose|y: int| 0 <= y < set.len() && set[y] && blocked(*s, v, y);
                assert(cur@[y]);
            }
        }
    }
    any
}

/// Checks the explorer's list against `certificate`: `true` only when the
/// list proves R41.3 and R41.1 for every run of the Endpoint
/// (`theorem_r41_3`).
pub fn certify(s: &System, list: &Vec<Config>) -> (r: bool)
    requires
        wf_sys(*s),
    ensures
        r ==> certificate(*s, cfgs(*list)),
        r ==> forall|c: Cfg| reachable(*s, c) ==> !deadlocked(*s, c) && within_capacity(*s, c),
{
    let init = initial_config(s.roles);
    if !contains(list, &init) {
        return false;
    }
    let mut i = 0;
    while i < list.len()
        invariant
            i <= list.len(),
            wf_sys(*s),
            in_list(cfgs(*list), initial(s.roles as int)),
            forall|a: int| 0 <= a < i ==> wf_cfg(s.roles as int, #[trigger] cfgs(*list)[a]) && within_capacity(*s, cfgs(*list)[a]) && !deadlocked(*s, cfgs(*list)[a]),
            forall|a: int, j: int| 0 <= a < i && 0 <= j < s.trans@.len() && enabled(s.roles as int, #[trigger] cfgs(*list)[a], #[trigger] s.trans@[j])
                ==> in_list(cfgs(*list), succ(s.roles as int, cfgs(*list)[a], s.trans@[j])),
        decreases list.len() - i,
    {
        let c = &list[i];
        assert(cfgs(*list)[i as int] == cfg(*c));
        if !wf_config(s, c) || !capacity_ok(s, c) || has_deadlock(s, c) {
            return false;
        }
        let mut j = 0;
        while j < s.trans.len()
            invariant
                j <= s.trans.len(),
                i < list.len(),
                wf_sys(*s),
                wf_cfg(s.roles as int, cfg(*c)),
                cfgs(*list)[i as int] == cfg(*c),
                forall|b: int| 0 <= b < j && enabled(s.roles as int, cfg(*c), #[trigger] s.trans@[b])
                    ==> in_list(cfgs(*list), succ(s.roles as int, cfg(*c), s.trans@[b])),
            decreases s.trans.len() - j,
        {
            if is_enabled(s, c, j) {
                let next = successor(s, c, j);
                if !contains(list, &next) {
                    return false;
                }
            }
            j += 1;
        }
        i += 1;
    }
    proof {
        theorem_r41_3(*s, cfgs(*list));
    }
    true
}

// ------------------------------------------- the verified exploration --

/// The most configurations the exploration visits before refusing.
pub const MAX_CONFIGURATIONS: usize = 200_000;

fn copy_config(c: &Config) -> (r: Config)
    ensures cfg(r) == cfg(*c),
{
    let states = copy_usize(&c.states);
    let mut chans: Vec<Vec<u32>> = Vec::new();
    let mut i = 0;
    while i < c.chans.len()
        invariant
            i <= c.chans.len(), chans.len() == i,
            forall|j: int| 0 <= j < i ==> (#[trigger] chans@[j])@ == c.chans@[j]@,
        decreases c.chans.len() - i,
    {
        chans.push(copy_from(&c.chans[i], 0));
        i += 1;
    }
    let r = Config { states, chans };
    assert forall|j: int| 0 <= j < cfg(r).chans.len() implies #[trigger] cfg(r).chans[j] == cfg(*c).chans[j] by {
        assert(cfg(r).chans[j] == chans@[j]@);
        assert(cfg(*c).chans[j] == c.chans@[j]@);
    }
    assert(cfg(r).chans =~= cfg(*c).chans);
    assert(cfg(r).states == cfg(*c).states);
    r
}

fn copy_usize(src: &Vec<usize>) -> (r: Vec<usize>)
    ensures r@ == src@,
{
    let mut out: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < src.len()
        invariant i <= src.len(), out@ == src@.subrange(0, i as int),
        decreases src.len() - i,
    {
        out.push(src[i]);
        i += 1;
        assert(out@ =~= src@.subrange(0, i as int));
    }
    assert(out@ =~= src@);
    out
}

/// The largest channel occupancy of a configuration.
fn occupancy(c: &Config) -> (k: usize)
    ensures
        forall|ch: int| 0 <= ch < cfg(*c).chans.len() ==> (#[trigger] cfg(*c).chans[ch]).len() <= k,
        k == 0 || exists|ch: int| 0 <= ch < cfg(*c).chans.len() && (#[trigger] cfg(*c).chans[ch]).len() == k,
{
    let mut k: usize = 0;
    let mut i = 0;
    while i < c.chans.len()
        invariant
            i <= c.chans.len(),
            forall|ch: int| 0 <= ch < i ==> (#[trigger] cfg(*c).chans[ch]).len() <= k,
            k == 0 || exists|ch: int| 0 <= ch < i && (#[trigger] cfg(*c).chans[ch]).len() == k,
        decreases c.chans.len() - i,
    {
        if c.chans[i].len() > k {
            k = c.chans[i].len();
            assert(cfg(*c).chans[i as int].len() == k);
        }
        i += 1;
    }
    k
}

/// The exploration of every reachable configuration of `s` from the
/// initial one, in breadth-first order: `Some((list, k))` when it
/// completes within `MAX_CONFIGURATIONS` with every configuration well
/// formed, within the capacity, and not deadlocked, in which case the
/// list is a certificate (R41.3) and `k` the largest occupancy any
/// reachable configuration reaches (R41.1, the universal bound, attained
/// by some listed configuration); `None` when a configuration is
/// deadlocked, exceeds the capacity, or the bound is reached (the
/// checker then reports the refusal with the explorer's own diagnosis).
pub fn explore_system(s: &System) -> (r: Option<(Vec<Config>, usize)>)
    requires wf_sys(*s),
    ensures
        r is Some ==> certificate(*s, cfgs(r.unwrap().0)),
        r is Some ==> forall|c: Cfg| reachable(*s, c) ==> !deadlocked(*s, c) && within_capacity(*s, c),
        r is Some ==> r.unwrap().1 <= s.capacity,
        r is Some ==> forall|i: int, ch: int| 0 <= i < cfgs(r.unwrap().0).len() && 0 <= ch < cfgs(r.unwrap().0)[i].chans.len()
            ==> (#[trigger] cfgs(r.unwrap().0)[i].chans[ch]).len() <= r.unwrap().1,
        r is Some ==> r.unwrap().1 == 0 || exists|i: int, ch: int| 0 <= i < cfgs(r.unwrap().0).len() && 0 <= ch < cfgs(r.unwrap().0)[i].chans.len()
            && (#[trigger] cfgs(r.unwrap().0)[i].chans[ch]).len() == r.unwrap().1,
{
    let init = initial_config(s.roles);
    // The initial configuration is checked like every other one.
    if !wf_config(s, &init) || !capacity_ok(s, &init) || has_deadlock(s, &init) {
        return None;
    }
    let mut list: Vec<Config> = Vec::new();
    list.push(init);
    let mut k: usize = 0;
    let mut next: usize = 0;
    assert(cfgs(list)[0] == initial(s.roles as int));
    assert forall|a: int, ch: int| 0 <= a < list.len() && 0 <= ch < cfgs(list)[a].chans.len() implies (#[trigger] cfgs(list)[a].chans[ch]).len() <= k by {
        assert(cfgs(list)[a] == initial(s.roles as int));
    }
    while next < list.len()
        invariant
            wf_sys(*s), next <= list.len(), list.len() <= MAX_CONFIGURATIONS,
            in_list(cfgs(list), initial(s.roles as int)),
            // Every visited configuration is well formed, within the
            // capacity, not deadlocked, and no larger than `k`.
            forall|a: int| 0 <= a < list.len() ==> wf_cfg(s.roles as int, #[trigger] cfgs(list)[a]) && within_capacity(*s, cfgs(list)[a]) && !deadlocked(*s, cfgs(list)[a]),
            forall|a: int, ch: int| 0 <= a < list.len() && 0 <= ch < cfgs(list)[a].chans.len() ==> (#[trigger] cfgs(list)[a].chans[ch]).len() <= k,
            k == 0 || exists|a: int, ch: int| 0 <= a < list.len() && 0 <= ch < cfgs(list)[a].chans.len() && (#[trigger] cfgs(list)[a].chans[ch]).len() == k,
            k <= s.capacity,
            // Every processed configuration's enabled steps lead into the list.
            forall|a: int, j: int| 0 <= a < next && 0 <= j < s.trans@.len() && enabled(s.roles as int, #[trigger] cfgs(list)[a], #[trigger] s.trans@[j])
                ==> in_list(cfgs(list), succ(s.roles as int, cfgs(list)[a], s.trans@[j])),
        decreases MAX_CONFIGURATIONS - next,
    {
        let c = copy_config(&list[next]);
        assert(cfg(c) == cfgs(list)[next as int]);
        let mut j = 0;
        while j < s.trans.len()
            invariant
                j <= s.trans.len(), wf_sys(*s), next < list.len(), list.len() <= MAX_CONFIGURATIONS,
                wf_cfg(s.roles as int, cfg(c)), cfg(c) == cfgs(list)[next as int],
                in_list(cfgs(list), initial(s.roles as int)),
                forall|a: int| 0 <= a < list.len() ==> wf_cfg(s.roles as int, #[trigger] cfgs(list)[a]) && within_capacity(*s, cfgs(list)[a]) && !deadlocked(*s, cfgs(list)[a]),
                forall|a: int, ch: int| 0 <= a < list.len() && 0 <= ch < cfgs(list)[a].chans.len() ==> (#[trigger] cfgs(list)[a].chans[ch]).len() <= k,
                k == 0 || exists|a: int, ch: int| 0 <= a < list.len() && 0 <= ch < cfgs(list)[a].chans.len() && (#[trigger] cfgs(list)[a].chans[ch]).len() == k,
                k <= s.capacity,
                forall|a: int, jj: int| 0 <= a < next && 0 <= jj < s.trans@.len() && enabled(s.roles as int, #[trigger] cfgs(list)[a], #[trigger] s.trans@[jj])
                    ==> in_list(cfgs(list), succ(s.roles as int, cfgs(list)[a], s.trans@[jj])),
                forall|jj: int| 0 <= jj < j && enabled(s.roles as int, cfg(c), #[trigger] s.trans@[jj])
                    ==> in_list(cfgs(list), succ(s.roles as int, cfg(c), s.trans@[jj])),
            decreases s.trans.len() - j,
        {
            if is_enabled(s, &c, j) {
                let n = successor(s, &c, j);
                if !contains(&list, &n) {
                    if list.len() >= MAX_CONFIGURATIONS {
                        return None;
                    }
                    if !wf_config(s, &n) || !capacity_ok(s, &n) || has_deadlock(s, &n) {
                        return None;
                    }
                    let occ = occupancy(&n);
                    if occ > k {
                        k = occ;
                    }
                    let ghost before = cfgs(list);
                    list.push(n);
                    proof {
                        assert(cfgs(list) =~= before.push(cfg(n)));
                        assert forall|a: int| 0 <= a < before.len() implies #[trigger] cfgs(list)[a] == before[a] by {}
                        assert(cfgs(list)[before.len() as int] == cfg(n));
                        assert(in_list(cfgs(list), cfg(n)));
                        assert forall|x: Cfg| in_list(before, x) implies in_list(cfgs(list), x) by {
                            let i = choose|i: int| 0 <= i < before.len() && before[i] == x;
                            assert(cfgs(list)[i] == x);
                        }
                        // The bound's witness: the new configuration when it
                        // raised `k`, the earlier one (unmoved) otherwise.
                        if k == occ && occ != 0 {
                            let ch = choose|ch: int| 0 <= ch < cfg(n).chans.len() && (#[trigger] cfg(n).chans[ch]).len() == occ;
                            assert(cfgs(list)[before.len() as int].chans[ch].len() == k);
                        } else if k != 0 {
                            let (a, ch) = choose|a: int, ch: int| 0 <= a < before.len() && 0 <= ch < before[a].chans.len() && (#[trigger] before[a].chans[ch]).len() == k;
                            assert(cfgs(list)[a] == before[a]);
                            assert(cfgs(list)[a].chans[ch].len() == k);
                        }
                        assert forall|a: int, ch: int| 0 <= a < list.len() && 0 <= ch < cfgs(list)[a].chans.len() implies (#[trigger] cfgs(list)[a].chans[ch]).len() <= k by {
                            if a < before.len() {
                                assert(cfgs(list)[a] == before[a]);
                            } else {
                                assert(cfgs(list)[a] == cfg(n));
                            }
                        }
                    }
                }
            }
            j += 1;
        }
        next += 1;
    }
    proof {
        theorem_r41_3(*s, cfgs(list));
    }
    Some((list, k))
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    fn t(role: usize, from: usize, send: bool, peer: usize, label: u32, to: usize) -> Trans {
        Trans { role, from, send, peer, label, to }
    }

    fn cfg_of(states: &[usize], chans: &[&[u32]]) -> Config {
        Config { states: states.to_vec(), chans: chans.iter().map(|c| c.to_vec()).collect() }
    }

    /// The request-response loop (`PingPong` without its stop branch):
    /// `a` sends 0 and awaits 1, `b` awaits 0 and sends 1; four
    /// configurations, closed, each channel at most one message (R41.3,
    /// R41.1).
    fn ping_pong() -> (System, Vec<Config>) {
        let trans = vec![t(0, 0, true, 1, 0, 1), t(0, 1, false, 1, 1, 0), t(1, 0, false, 0, 0, 1), t(1, 1, true, 0, 1, 0)];
        let list = vec![
            cfg_of(&[0, 0], &[&[], &[], &[], &[]]),
            cfg_of(&[1, 0], &[&[], &[0], &[], &[]]),
            cfg_of(&[1, 1], &[&[], &[], &[], &[]]),
            cfg_of(&[1, 0], &[&[], &[], &[1], &[]]),
        ];
        (System { roles: 2, capacity: 1, trans }, list)
    }

    /// R41.3: the explorer's set of a request-response loop is a
    /// certificate.
    #[test]
    fn r41_3_the_request_response_loop_is_certified() {
        let (s, list) = ping_pong();
        assert!(certify(&s, &list));
    }

    /// R41.3: a list that misses a successor is not closed and is refused;
    /// one that misses the initial configuration is refused.
    #[test]
    fn r41_3_an_unclosed_or_rootless_list_is_refused() {
        let (s, mut list) = ping_pong();
        let last = list.pop().unwrap();
        assert!(!certify(&s, &list));
        list.push(last);
        list.remove(0);
        assert!(!certify(&s, &list));
    }

    /// R41.1 through the certificate: a listed channel content over the
    /// capacity is refused.
    #[test]
    fn r41_1_a_channel_over_the_capacity_is_refused() {
        let (mut s, list) = ping_pong();
        s.capacity = 0;
        assert!(!certify(&s, &list));
    }

    /// R41.3: two roles that each await the other's first message, with
    /// nothing in transit, are a waiting set; the initial configuration
    /// alone is a closed list (no operation is enabled) and is refused.
    #[test]
    fn r41_3_two_roles_awaiting_each_other_are_refused() {
        let trans = vec![t(0, 0, false, 1, 0, 1), t(1, 0, false, 0, 1, 1)];
        let s = System { roles: 2, capacity: 1, trans };
        let list = vec![cfg_of(&[0, 0], &[&[], &[], &[], &[]])];
        assert!(!certify(&s, &list));
    }

    /// R41.3's set is partial: a waiting pair beside a live request-
    /// response pair is refused although some role can always move (the
    /// explorer's own check, "no role can move", would not see it).
    #[test]
    fn r41_3_a_waiting_pair_beside_a_live_pair_is_refused() {
        let mut trans = vec![t(0, 0, true, 1, 0, 1), t(0, 1, false, 1, 1, 0), t(1, 0, false, 0, 0, 1), t(1, 1, true, 0, 1, 0)];
        trans.push(t(2, 0, false, 3, 2, 1));
        trans.push(t(3, 0, false, 2, 3, 1));
        let n = 4;
        let empty: Vec<Vec<u32>> = vec![vec![]; n * n];
        let mut c1 = empty.clone();
        c1[1] = vec![0];
        let mut c3 = empty.clone();
        c3[n] = vec![1];
        let list = vec![
            Config { states: vec![0, 0, 0, 0], chans: empty.clone() },
            Config { states: vec![1, 0, 0, 0], chans: c1 },
            Config { states: vec![1, 1, 0, 0], chans: empty.clone() },
            Config { states: vec![1, 0, 0, 0], chans: c3 },
        ];
        let s = System { roles: n, capacity: 1, trans };
        assert!(!certify(&s, &list));
        // Without the waiting pair's operations the same list is certified.
        let s2 = System { roles: n, capacity: 1, trans: s.trans[..4].to_vec() };
        assert!(certify(&s2, &list));
    }
}
