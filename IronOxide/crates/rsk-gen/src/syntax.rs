//! The `rsk::app!` surface syntax (design.md example; R21.1), parsed with
//! `syn` into a [`Declaration`] plus a span map, so that the front end can
//! report every Generator diagnostic at the source span of the offending
//! item (R21.4, R3.1). The CLI uses the same parser on the declaration
//! crate's source (R24.3), so the macro and the CLI cannot diverge.
//!
//! ```text
//! rsk::app! {
//!     target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 200.ms(),
//!     safe_state = crate::report_and_exit, log_capacity = 32, fp_flush_to_zero = false;
//!
//!     partition part0 (level = A, crate = p1_demo_part0, code = 32.KiB(), ram = 32.KiB(),
//!                      stack = 4.KiB(), fault = RESTART_PARTITION, overrun = END_JOB,
//!                      deadline_miss = RECORD_ONLY, mit_threshold = 10) {
//!         peripherals = [];
//!         resource state: u32 = 0;
//!         task control (periodic, period = 10.ms(), deadline = 10.ms(),
//!                       budget = 2000000.cycles(), priority = 7, resources = [state]);
//!         task imu (sporadic, binds = GPIOTE, mit = 1.ms(), mit_policy = defer,
//!                   deadline = 1.ms(), budget = 40.us(), priority = 6, fpu);
//!         signal tick;
//!     }
//! }
//! ```
//!
//! Time quantities are `N.unit()` with unit in cycles, ticks, ns, us, ms,
//! s, min, h (R21.2: no unit, no quantity); sizes are `N.B()`, `N.KiB()`,
//! `N.MiB()`.

use proc_macro2::{Span, TokenStream};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, bracketed, parenthesized, Ident, LitInt, Token};

use crate::decl::*;

/// A parse or validation error with its span.
#[derive(Clone, Debug)]
pub struct SpannedError {
    pub span: Span,
    pub message: String,
}

/// Where every item and field was declared, for diagnostics.
#[derive(Clone, Debug, Default)]
pub struct SpanMap {
    entries: Vec<(ItemRef, Span)>,
}

impl SpanMap {
    fn add(&mut self, item: ItemRef, span: Span) {
        self.entries.push((item, span));
    }

    /// The span of an item, or of its enclosing item, or the call site.
    pub fn span_of(&self, item: &ItemRef) -> Span {
        if let Some((_, s)) = self.entries.iter().find(|(i, _)| i == item) {
            return *s;
        }
        let parent = match item {
            ItemRef::Field(_) => ItemRef::Declaration,
            ItemRef::PartitionField(p, _) => ItemRef::Partition(p.clone()),
            ItemRef::TaskField(p, t, _) => ItemRef::Task(p.clone(), t.clone()),
            ItemRef::Task(p, _) | ItemRef::Resource(p, _) | ItemRef::Signal(p, _) | ItemRef::Peripheral(p, _) => {
                ItemRef::Partition(p.clone())
            }
            _ => return Span::call_site(),
        };
        self.span_of(&parent)
    }

    /// Resolves Generator diagnostics to spans.
    pub fn resolve(&self, diagnostics: &[Diagnostic]) -> Vec<SpannedError> {
        diagnostics
            .iter()
            .map(|d| SpannedError {
                span: self.span_of(&d.item),
                message: format!("[{}] {}", d.rule, d.message),
            })
            .collect()
    }
}

/// A value on the right of `name = ...`.
#[derive(Clone, Debug)]
enum Val {
    Int(u64, Span),
    /// `N.unit()`: a time or a size.
    Quantity(u64, String, Span),
    Ident(String, Span),
    Path(String, Span),
    List(Vec<(String, Span)>, Span),
}

impl Val {
    fn span(&self) -> Span {
        match self {
            Val::Int(_, s) | Val::Quantity(_, _, s) | Val::Ident(_, s) | Val::Path(_, s) | Val::List(_, s) => *s,
        }
    }
}

fn err<T>(span: Span, msg: impl Into<String>) -> syn::Result<T> {
    Err(syn::Error::new(span, msg.into()))
}

fn parse_val(input: ParseStream) -> syn::Result<Val> {
    if input.peek(syn::LitBool) {
        let lit: syn::LitBool = input.parse()?;
        return Ok(Val::Ident(lit.value.to_string(), lit.span()));
    }
    if input.peek(LitInt) {
        let lit: LitInt = input.parse()?;
        let n: u64 = lit.base10_parse()?;
        if input.peek(Token![.]) {
            input.parse::<Token![.]>()?;
            let unit: Ident = input.parse()?;
            let content;
            parenthesized!(content in input);
            if !content.is_empty() {
                return err(content.span(), "a unit takes no arguments: write `10.ms()`");
            }
            return Ok(Val::Quantity(n, unit.to_string(), lit.span()));
        }
        return Ok(Val::Int(n, lit.span()));
    }
    if input.peek(syn::token::Bracket) {
        let content;
        let bracket = bracketed!(content in input);
        let items: Punctuated<Ident, Token![,]> = content.parse_terminated(Ident::parse, Token![,])?;
        return Ok(Val::List(items.iter().map(|i| (i.to_string(), i.span())).collect(), bracket.span.join()));
    }
    let path: syn::Path = input.parse()?;
    let span = path.segments.first().map(|s| s.ident.span()).unwrap_or_else(Span::call_site);
    if path.segments.len() == 1 && path.leading_colon.is_none() {
        return Ok(Val::Ident(path.segments[0].ident.to_string(), span));
    }
    let text = path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    Ok(Val::Path(if path.leading_colon.is_some() { format!("::{text}") } else { text }, span))
}

/// `name = value` or a bare flag `name`.
struct Attr {
    name: String,
    span: Span,
    value: Option<Val>,
}

fn parse_attrs(input: ParseStream) -> syn::Result<Vec<Attr>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let name: Ident = input.call(Ident::parse_any)?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(parse_val(input)?)
        } else {
            None
        };
        out.push(Attr {
            name: name.to_string(),
            span: name.span(),
            value,
        });
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else {
            break;
        }
    }
    Ok(out)
}

struct Attrs {
    items: Vec<Attr>,
    owner: Span,
    what: &'static str,
}

impl Attrs {
    fn take(&mut self, name: &str) -> Option<Attr> {
        let i = self.items.iter().position(|a| a.name == name)?;
        Some(self.items.remove(i))
    }

    fn required(&mut self, name: &str) -> syn::Result<Attr> {
        match self.take(name) {
            Some(a) => Ok(a),
            None => err(self.owner, format!("{} requires `{name} = ...` (R21.3; PR-27: the required attributes are declared up front)", self.what)),
        }
    }

    fn flag(&mut self, name: &str) -> syn::Result<bool> {
        match self.take(name) {
            None => Ok(false),
            Some(Attr { value: None, .. }) => Ok(true),
            Some(a) => err(a.span, format!("`{name}` is a flag and takes no value")),
        }
    }

    fn finish(self) -> syn::Result<()> {
        if let Some(a) = self.items.first() {
            let hint = match a.name.as_str() {
                "priority" => " (PR-03: a Task declares exactly one Priority)",
                "when" | "condition" | "guard" | "barrier" => " (PR-13: the Ravenscar Profile_Variant has no release condition)",
                _ => "",
            };
            return err(a.span, format!("unknown or repeated attribute `{}` of {}{hint}", a.name, self.what));
        }
        Ok(())
    }
}

fn time(a: Attr) -> syn::Result<Time> {
    match a.value {
        Some(Val::Quantity(n, unit, span)) => match TimeUnit::parse(&unit) {
            Some(u) => Ok(Time { value: n, unit: u }),
            None => err(span, format!("`{unit}` is not a time unit (cycles, ticks, ns, us, ms, s, min, h)")),
        },
        Some(v) => err(v.span(), format!("`{}` needs a time quantity with an explicit unit, such as `10.ms()` (R21.2)", a.name)),
        None => err(a.span, format!("`{}` needs a value", a.name)),
    }
}

fn size(a: Attr) -> syn::Result<Bytes> {
    match a.value {
        Some(Val::Quantity(n, unit, span)) => {
            let mult = match unit.as_str() {
                "B" => 1u64,
                "KiB" => 1024,
                "MiB" => 1024 * 1024,
                _ => return err(span, format!("`{unit}` is not a size unit (B, KiB, MiB)")),
            };
            n.checked_mul(mult).ok_or_else(|| syn::Error::new(span, "size overflow"))
        }
        Some(v) => err(v.span(), format!("`{}` needs a size with a unit, such as `32.KiB()`", a.name)),
        None => err(a.span, format!("`{}` needs a value", a.name)),
    }
}

fn int(a: Attr) -> syn::Result<u64> {
    match a.value {
        Some(Val::Int(n, _)) => Ok(n),
        Some(v) => err(v.span(), format!("`{}` needs an integer", a.name)),
        None => err(a.span, format!("`{}` needs a value", a.name)),
    }
}

fn ident(a: Attr) -> syn::Result<(String, Span)> {
    match a.value {
        Some(Val::Ident(s, span)) => Ok((s, span)),
        Some(v) => err(v.span(), format!("`{}` needs an identifier", a.name)),
        None => err(a.span, format!("`{}` needs a value", a.name)),
    }
}

fn path(a: Attr) -> syn::Result<String> {
    match a.value {
        Some(Val::Ident(s, _)) | Some(Val::Path(s, _)) => Ok(s),
        Some(v) => err(v.span(), format!("`{}` needs a path", a.name)),
        None => err(a.span, format!("`{}` needs a value", a.name)),
    }
}

fn list(a: Attr) -> syn::Result<Vec<(String, Span)>> {
    match a.value {
        Some(Val::List(items, _)) => Ok(items),
        Some(v) => err(v.span(), format!("`{}` needs a list `[a, b]`", a.name)),
        None => err(a.span, format!("`{}` needs a value", a.name)),
    }
}

/// The parsed declaration with its span map.
pub struct Parsed {
    pub decl: Declaration,
    pub spans: SpanMap,
}

impl Parse for Parsed {
    fn parse(input: ParseStream) -> syn::Result<Parsed> {
        let mut spans = SpanMap::default();
        let call = input.span();
        spans.add(ItemRef::Declaration, call);
        // Header: attributes up to `;`.
        let header = parse_attrs(input)?;
        input.parse::<Token![;]>()?;
        let mut h = Attrs {
            items: header,
            owner: call,
            what: "the app header",
        };
        for a in &h.items {
            spans.add(ItemRef::Field(a.name.clone()), a.span);
        }
        let (target, _) = ident(h.required("target")?)?;
        let variant = match h.take("profile") {
            Some(a) => ident(a)?.0,
            None => crate::profile_version::PROFILE_DEFAULT_VARIANT.to_string(),
        };
        let operating_duration = time(h.required("operating_duration")?)?;
        let safe_state = path(h.required("safe_state")?)?;
        let log_capacity = match h.take("log_capacity") {
            Some(a) => int(a)? as u32,
            None => 32,
        };
        // Flush-to-zero for every Job's floating-point context (FPDSCR.FZ,
        // the workaround of Cortex-M4 erratum 839676; task 3.4): `true` or
        // `false`, `false` by default.
        let fp_flush_to_zero = match h.take("fp_flush_to_zero") {
            Some(a) => {
                let (v, span) = ident(a)?;
                match v.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return err(span, "`fp_flush_to_zero` is `true` or `false`".to_string()),
                }
            }
            None => false,
        };
        h.finish()?;

        let mut partitions = Vec::new();
        let mut endpoints = Vec::new();
        let mut protocols = Vec::new();
        let mut chains = Vec::new();
        let mut ppi = Vec::new();
        while !input.is_empty() {
            let kw: Ident = input.parse()?;
            if kw == "ppi" {
                // docs/pacemaker PM33.1: `ppi NAME (event = P[OFF], task =
                // P[OFF], fork = P[OFF]);`, a fixed PPI channel.
                let name: Ident = input.parse()?;
                let cname = name.to_string();
                spans.add(ItemRef::Ppi(cname.clone()), name.span());
                let content;
                parenthesized!(content in input);
                let (mut event, mut task, mut fork) = (None, None, None);
                while !content.is_empty() {
                    let key: Ident = content.parse()?;
                    content.parse::<Token![=]>()?;
                    let peripheral: Ident = content.parse()?;
                    let index;
                    bracketed!(index in content);
                    let lit: LitInt = index.parse()?;
                    if !index.is_empty() {
                        return err(index.span(), "a register is `PERIPHERAL[OFFSET]` with one integer offset");
                    }
                    let offset: u32 = lit.base10_parse()?;
                    let reg = PpiRef { peripheral: peripheral.to_string(), offset };
                    let slot = match key.to_string().as_str() {
                        "event" => &mut event,
                        "task" => &mut task,
                        "fork" => &mut fork,
                        other => return err(key.span(), format!("unknown attribute `{other}` of a ppi channel (event, task, fork)")),
                    };
                    if slot.replace(reg).is_some() {
                        return err(key.span(), format!("`{key}` is given twice"));
                    }
                    if content.peek(Token![,]) {
                        content.parse::<Token![,]>()?;
                    } else {
                        break;
                    }
                }
                if !content.is_empty() {
                    return err(content.span(), "expected `,` between the attributes of a ppi channel");
                }
                let Some(event) = event else { return err(name.span(), format!("ppi channel `{cname}` requires `event = PERIPHERAL[OFFSET]`")) };
                let Some(task) = task else { return err(name.span(), format!("ppi channel `{cname}` requires `task = PERIPHERAL[OFFSET]`")) };
                input.parse::<Token![;]>()?;
                ppi.push(PpiDecl { name: cname, event, task, fork });
                continue;
            }
            if kw == "protocol" {
                // Task 21.1: `protocol NAME { <global type> }`, the body in the
                // Protocol_Checker's syntax, kept as text.
                let name: Ident = input.parse()?;
                let pname = name.to_string();
                spans.add(ItemRef::Protocol(pname.clone()), name.span());
                let body;
                braced!(body in input);
                let tokens: TokenStream = body.parse()?;
                protocols.push(ProtocolDecl { name: pname, text: tokens.to_string() });
                continue;
            }
            if kw == "endpoint" {
                // Task 21.2: `endpoint NAME: TYPE { ROLE => PART.TASK, .. }
                // capacity = N, payload = BYTES;`; task 21.1: `endpoint NAME:
                // protocol PNAME { .. } ..` for a Protocol-governed one.
                let name: Ident = input.parse()?;
                let ename = name.to_string();
                spans.add(ItemRef::Endpoint(ename.clone()), name.span());
                input.parse::<Token![:]>()?;
                let (protocol, ty): (Option<String>, String) = if input.peek(Ident) && input.fork().parse::<Ident>().map(|i| i == "protocol").unwrap_or(false) {
                    let _kw: Ident = input.parse()?;
                    let p: Ident = input.parse()?;
                    (Some(p.to_string()), String::new())
                } else {
                    let ty: syn::Type = input.parse()?;
                    (None, tokens_to_string(&ty).replace(' ', ""))
                };
                let body;
                braced!(body in input);
                let mut roles = Vec::new();
                while !body.is_empty() {
                    let role: Ident = body.parse()?;
                    body.parse::<Token![=>]>()?;
                    let part: Ident = body.parse()?;
                    body.parse::<Token![.]>()?;
                    let task: Ident = body.parse()?;
                    // Task 21.4 (R42.4): `validate = PATH` (plain form) or
                    // `validate = { LABEL = PATH, .. }` (Protocol form); task
                    // 21.5 (R40.2): `ops = N`.
                    let mut validate = Vec::new();
                    let mut ops: u32 = 1;
                    let mut after = Vec::new();
                    while body.peek(Ident) {
                        let kw: Ident = body.parse()?;
                        body.parse::<Token![=]>()?;
                        if kw == "validate" {
                            if body.peek(syn::token::Brace) {
                                let inner;
                                braced!(inner in body);
                                while !inner.is_empty() {
                                    let label: Ident = inner.parse()?;
                                    inner.parse::<Token![=]>()?;
                                    let path: syn::Path = inner.parse()?;
                                    validate.push((Some(label.to_string()), tokens_to_string(&path).replace(' ', "")));
                                    if !inner.is_empty() {
                                        inner.parse::<Token![,]>()?;
                                    }
                                }
                            } else {
                                let path: syn::Path = body.parse()?;
                                validate.push((None, tokens_to_string(&path).replace(' ', "")));
                            }
                        } else if kw == "ops" {
                            let n: syn::LitInt = body.parse()?;
                            ops = n.base10_parse::<u32>()?;
                        } else if kw == "after" {
                            // Task 21.6 (R41.3, A-P8): `after = ENDPOINT`.
                            let gate: Ident = body.parse()?;
                            after.push(gate.to_string());
                        } else {
                            return err(kw.span(), "expected `validate`, `ops`, or `after` after the role's Task");
                        }
                    }
                    roles.push(RoleDecl { role: role.to_string(), partition: part.to_string(), task: task.to_string(), validate, ops, after });
                    if !body.is_empty() {
                        body.parse::<Token![,]>()?;
                    }
                }
                let attrs = parse_attrs(input)?;
                input.parse::<Token![;]>()?;
                let mut ea = Attrs { items: attrs, owner: name.span(), what: "an endpoint" };
                let capacity = int(ea.required("capacity")?)? as u32;
                let payload = int(ea.required("payload")?)? as u32;
                ea.finish()?;
                endpoints.push(EndpointDecl { name: ename, ty, protocol, roles, capacity, payload });
                continue;
            }
            if kw == "chain" {
                // Task 21.5 (R41.4): `chain NAME { PART.TASK -> PART.TASK [-> ..] }
                // deadline = T;`.
                let name: Ident = input.parse()?;
                let cname = name.to_string();
                spans.add(ItemRef::Chain(cname.clone()), name.span());
                let body;
                braced!(body in input);
                let mut tasks = Vec::new();
                loop {
                    let part: Ident = body.parse()?;
                    body.parse::<Token![.]>()?;
                    let task: Ident = body.parse()?;
                    tasks.push((part.to_string(), task.to_string()));
                    if body.is_empty() {
                        break;
                    }
                    body.parse::<Token![->]>()?;
                }
                let attrs = parse_attrs(input)?;
                input.parse::<Token![;]>()?;
                let mut ca = Attrs { items: attrs, owner: name.span(), what: "a chain" };
                let deadline = time(ca.required("deadline")?)?;
                ca.finish()?;
                chains.push(ChainDecl { name: cname, tasks, deadline });
                continue;
            }
            if kw != "partition" {
                return err(kw.span(), "expected `partition`, `endpoint`, `protocol`, or `chain`");
            }
            let name: Ident = input.parse()?;
            let pname = name.to_string();
            spans.add(ItemRef::Partition(pname.clone()), name.span());
            let content;
            parenthesized!(content in input);
            let attrs = parse_attrs(&content)?;
            for a in &attrs {
                spans.add(ItemRef::PartitionField(pname.clone(), a.name.clone()), a.span);
            }
            let mut pa = Attrs {
                items: attrs,
                owner: name.span(),
                what: "a partition",
            };
            let (level_s, level_span) = ident(pa.required("level")?)?;
            let level = Level::parse(&level_s).ok_or_else(|| syn::Error::new(level_span, "the Criticality_Level is one of A, B, C, D, E (R15.1)"))?;
            let krate = match pa.take("crate") {
                Some(a) => path(a)?,
                None => pname.clone(),
            };
            let code = size(pa.required("code")?)?;
            let ram = size(pa.required("ram")?)?;
            let stack = size(pa.required("stack")?)?;
            let response = |a: Attr| -> syn::Result<Response> {
                let (s, span) = ident(a)?;
                Response::parse(&s).ok_or_else(|| {
                    syn::Error::new(
                        span,
                        format!("`{s}` is not a response of R14.5 (END_JOB, RESTART_PARTITION, STOP_PARTITION, SAFE_STATE, RECORD_ONLY, RECORD_AND_CONTINUE)"),
                    )
                })
            };
            let fault = response(pa.required("fault")?)?;
            let overrun = response(pa.required("overrun")?)?;
            let deadline_miss = response(pa.required("deadline_miss")?)?;
            let mit_threshold = int(pa.required("mit_threshold")?)? as u32;
            let init_arena = match pa.take("init_arena") {
                Some(a) => size(a)?,
                None => 0,
            };
            pa.finish()?;

            let body;
            braced!(body in input);
            let mut peripherals = Vec::new();
            let mut resources = Vec::new();
            let mut tasks: Vec<TaskDecl> = Vec::new();
            let mut signals = Vec::new();
            // (task name, state, span of the task name): attached after the loop.
            let mut states: Vec<(String, StateDecl, Span)> = Vec::new();
            while !body.is_empty() {
                let kw: Ident = body.parse()?;
                match kw.to_string().as_str() {
                    "peripherals" => {
                        body.parse::<Token![=]>()?;
                        let v = parse_val(&body)?;
                        let items = match v {
                            Val::List(items, _) => items,
                            v => return err(v.span(), "`peripherals = [..]` takes a list"),
                        };
                        for (n, s) in items {
                            spans.add(ItemRef::Peripheral(pname.clone(), n.clone()), s);
                            peripherals.push(n);
                        }
                        body.parse::<Token![;]>()?;
                    }
                    "resource" => {
                        let rname: Ident = body.parse()?;
                        spans.add(ItemRef::Resource(pname.clone(), rname.to_string()), rname.span());
                        body.parse::<Token![:]>()?;
                        let ty: syn::Type = body.parse()?;
                        body.parse::<Token![=]>()?;
                        let init: syn::Expr = body.parse()?;
                        // R46.4: `invariant |v: T| <predicate>` (a Rust closure, read
                        // by the App_Prover as the Resource's invariant).
                        // The body is Pearlite (`v@ <= 1`), not Rust, so it is kept
                        // as tokens up to the `;`.
                        let invariant = if body.peek(Ident) && body.fork().parse::<Ident>().is_ok_and(|k| k == "invariant") {
                            let _: Ident = body.parse()?;
                            body.parse::<Token![|]>()?;
                            let param: Ident = body.parse()?;
                            body.parse::<Token![:]>()?;
                            let pty: syn::Type = body.parse()?;
                            body.parse::<Token![|]>()?;
                            let mut pred = TokenStream::new();
                            while !body.peek(Token![;]) {
                                if body.is_empty() {
                                    return err(rname.span(), "a Resource invariant is `|v: TYPE| PREDICATE;` (R46.4)");
                                }
                                let tt: proc_macro2::TokenTree = body.parse()?;
                                pred.extend(std::iter::once(tt));
                            }
                            if pred.is_empty() {
                                return err(rname.span(), "a Resource invariant is `|v: TYPE| PREDICATE;` (R46.4)");
                            }
                            Some(format!("|{}: {}| {}", param, tokens_to_string(&pty), pred))
                        } else {
                            None
                        };
                        body.parse::<Token![;]>()?;
                        resources.push(ResourceDecl {
                            name: rname.to_string(),
                            invariant,
                            ty: tokens_to_string(&ty),
                            init: tokens_to_string(&init),
                            placement: ResourcePlacement::Core(0),
                        });
                    }
                    "state" => {
                        // PR-25: `state <task>.<name>: <type> = <init>;`
                        let tname: Ident = body.parse()?;
                        body.parse::<Token![.]>()?;
                        let sname: Ident = body.parse()?;
                        spans.add(ItemRef::TaskField(pname.clone(), tname.to_string(), sname.to_string()), sname.span());
                        body.parse::<Token![:]>()?;
                        let ty: syn::Type = body.parse()?;
                        body.parse::<Token![=]>()?;
                        let init: syn::Expr = body.parse()?;
                        body.parse::<Token![;]>()?;
                        states.push((tname.to_string(), StateDecl { name: sname.to_string(), ty: tokens_to_string(&ty), init: tokens_to_string(&init) }, tname.span()));
                    }
                    "signal" => {
                        let sname: Ident = body.parse()?;
                        spans.add(ItemRef::Signal(pname.clone(), sname.to_string()), sname.span());
                        body.parse::<Token![;]>()?;
                        signals.push(SignalDecl { name: sname.to_string() });
                    }
                    "task" => {
                        let tname: Ident = body.parse()?;
                        let tn = tname.to_string();
                        spans.add(ItemRef::Task(pname.clone(), tn.clone()), tname.span());
                        let content;
                        parenthesized!(content in body);
                        let attrs = parse_attrs(&content)?;
                        for a in &attrs {
                            spans.add(ItemRef::TaskField(pname.clone(), tn.clone(), a.name.clone()), a.span);
                        }
                        body.parse::<Token![;]>()?;
                        let mut ta = Attrs {
                            items: attrs,
                            owner: tname.span(),
                            what: "a task",
                        };
                        let periodic = ta.flag("periodic")?;
                        let sporadic = ta.flag("sporadic")?;
                        if periodic == sporadic {
                            return err(tname.span(), "a task is `periodic` or `sporadic` (PR-10)");
                        }
                        let release = if periodic {
                            let period = time(ta.required("period")?)?;
                            let offset = match ta.take("offset") {
                                Some(a) => Some(time(a)?),
                                None => None,
                            };
                            for forbidden in ["binds", "signal", "mit", "mit_policy"] {
                                if let Some(a) = ta.take(forbidden) {
                                    return err(a.span, format!("a periodic task has no `{forbidden}` (PR-11: a Periodic_Task has no Release_Source)"));
                                }
                            }
                            Release::Periodic { period, offset }
                        } else {
                            let mit = time(ta.required("mit")?)?;
                            let policy = match ta.take("mit_policy") {
                                Some(a) => {
                                    let (s, span) = ident(a)?;
                                    MitPolicy::parse(&s).ok_or_else(|| syn::Error::new(span, "`mit_policy` is `defer` or `discard` (R10.2)"))?
                                }
                                None => MitPolicy::Defer,
                            };
                            let binds = ta.take("binds");
                            let signal = ta.take("signal");
                            let endpoint = ta.take("endpoint");
                            let source = match (binds, signal, endpoint) {
                                (Some(b), None, None) => Source::Interrupt(ident(b)?.0),
                                (None, Some(s), None) => Source::Signal(ident(s)?.0),
                                (None, None, Some(e)) => Source::Endpoint(ident(e)?.0),
                                (Some(b), _, _) => return err(b.span, "a task has exactly one Release_Source: `binds`, `signal`, or `endpoint` (PR-11, R22.4)"),
                                (None, Some(s), Some(_)) => return err(s.span, "a task has exactly one Release_Source: `binds`, `signal`, or `endpoint` (PR-11, R22.4)"),
                                (None, None, None) => return err(tname.span(), "a sporadic task declares its Release_Source: `binds = PERIPHERAL`, `signal = NAME`, or `endpoint = NAME` (PR-11, R22.4)"),
                            };
                            if let Some(a) = ta.take("period").or_else(|| ta.take("offset")) {
                                return err(a.span, "a sporadic task has an `mit`, not a period or offset");
                            }
                            Release::Sporadic { mit, policy, source }
                        };
                        let deadline = time(ta.required("deadline")?)?;
                        let budget = time(ta.required("budget")?)?;
                        let priority = int(ta.required("priority")?)?;
                        let core = match ta.take("core") {
                            Some(a) => int(a)?,
                            None => 0,
                        };
                        let fpu = ta.flag("fpu")?;
                        let resources = match ta.take("resources") {
                            Some(a) => list(a)?.into_iter().map(|(n, _)| n).collect(),
                            None => vec![],
                        };
                        let raises = match ta.take("raises") {
                            Some(a) => list(a)?.into_iter().map(|(n, _)| n).collect(),
                            None => vec![],
                        };
                        ta.finish()?;
                        tasks.push(TaskDecl {
                            name: tn,
                            release,
                            deadline,
                            budget,
                            priority: priority.min(255) as u8,
                            core: core.min(255) as u8,
                            fpu,
                            resources,
                            raises,
                            state: vec![],
                        });
                    }
                    other => return err(kw.span(), format!("unknown item `{other}` in a partition (peripherals, resource, state, task, signal)")),
                }
            }
            for (tname, state, span) in states {
                match tasks.iter_mut().find(|t| t.name == tname) {
                    Some(t) => t.state.push(state),
                    None => return err(span, format!("`state {tname}.{}` names no task `{tname}` of partition `{pname}` (PR-25: per-Task state belongs to a declared Task)", state.name)),
                }
            }
            partitions.push(PartitionDecl {
                name: pname,
                krate,
                level,
                code,
                ram,
                stack,
                fault,
                overrun,
                deadline_miss,
                mit_threshold,
                peripherals,
                resources,
                tasks,
                signals,
                init_arena,
            });
        }
        Ok(Parsed {
            decl: Declaration {
                target,
                variant,
                operating_duration,
                safe_state,
                log_capacity,
                fp_flush_to_zero,
                partitions,
                endpoints,
                protocols,
                chains,
                ppi,
            },
            spans,
        })
    }
}

fn tokens_to_string<T: quote_to_tokens::ToTokens>(t: &T) -> String {
    let mut ts = TokenStream::new();
    t.to_tokens(&mut ts);
    ts.to_string()
}

/// `syn`'s `ToTokens` re-exported under a private name (quote is not a
/// dependency of the core).
mod quote_to_tokens {
    pub use syn::__private::ToTokens;
}

/// Parses the body of an `rsk::app! { ... }` invocation.
pub fn parse_app(tokens: TokenStream) -> syn::Result<Parsed> {
    syn::parse2(tokens)
}

/// Finds the `rsk::app! { ... }` (or `app! { ... }`) invocation in a Rust
/// source file and parses it (the CLI path, R24.3).
pub fn parse_source(source: &str) -> Result<Parsed, String> {
    let file: syn::File = syn::parse_str(source).map_err(|e| format!("cannot parse the declaration source: {e}"))?;
    for item in &file.items {
        if let syn::Item::Macro(m) = item {
            let last = m.mac.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
            if last == "app" {
                return parse_app(m.mac.tokens.clone()).map_err(|e| format!("rsk::app!: {e}"));
            }
        }
    }
    Err("no `rsk::app! { ... }` invocation found in the source".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = r#"
        target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 200.ms(),
        safe_state = crate::report_and_exit, log_capacity = 32;
        partition part0 (level = A, crate = p1_demo_part0, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                         fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY, mit_threshold = 10) {
            peripherals = [];
            resource state: u32 = 0;
            task control (periodic, period = 10.ms(), offset = 0.ms(), deadline = 10.ms(), budget = 9.ms(), priority = 7, resources = [state]);
            task sensor (periodic, period = 20.ms(), deadline = 20.ms(), budget = 19.ms(), priority = 5, resources = [state]);
        }
        partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                         fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY, mit_threshold = 10) {
            task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 39.ms(), priority = 2);
        }
    "#;

    /// R21.1, R15.1: the surface syntax declares the Target, Profile_Variant,
    /// Partitions (name, Criticality_Level, region sizes, peripherals, the
    /// three responses and the MIT_Violation threshold), Tasks and Resources,
    /// and the result generates.
    #[test]
    fn parses_the_demo_declaration() {
        let parsed = parse_app(DEMO.parse().unwrap()).unwrap();
        let d = parsed.decl;
        assert_eq!(d.target, "qemu_mps2_an386");
        assert_eq!(d.partitions.len(), 2);
        assert_eq!(d.partitions[0].tasks[0].resources, vec!["state".to_string()]);
        assert_eq!(d.partitions[0].resources[0].ty, "u32");
        assert_eq!(d.partitions[1].tasks[0].priority, 2);
        if let Err(errs) = crate::generate(&d) {
            panic!("{}", errs.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"));
        }
    }

    /// The demo on the nRF52840 with two peripherals a channel can connect.
    fn ppi_demo(channels: &str) -> String {
        let src = DEMO.replace("target = qemu_mps2_an386", "target = nrf52840").replace("peripherals = [];", "peripherals = [TIMER1, GPIOTE];");
        assert_ne!(src, DEMO);
        format!("{src}\n{channels}")
    }

    fn rules(src: &str) -> Vec<(String, String)> {
        let parsed = parse_app(src.parse().unwrap()).unwrap();
        match crate::generate(&parsed.decl) {
            Ok(_) => vec![],
            Err(errs) => errs.iter().map(|e| (e.rule.clone(), e.item.to_string())).collect(),
        }
    }

    /// The Task_Model's `ppi` value for the two channels of the test below,
    /// as the Generator prints it. `rsk-model`'s test
    /// `ppi_channels_parse_and_meet_the_schema` reads the same text.
    const PPI_MODEL: &str = r#"[{"id":"compare_sets_pin","channel":0,"event":{"peripheral":"TIMER1","offset":320,"address":"0x40009140"},"task":{"peripheral":"GPIOTE","offset":48,"address":"0x40006030"},"derived":["channel","address"]},{"id":"edge_captures","channel":1,"event":{"peripheral":"GPIOTE","offset":256,"address":"0x40006100"},"task":{"peripheral":"TIMER1","offset":64,"address":"0x40009040"},"fork":{"peripheral":"TIMER1","offset":8,"address":"0x40009008"},"derived":["channel","address"]}]"#;

    /// PM33.1 (docs/pacemaker): a fixed PPI channel names one event of a
    /// peripheral, one task of a peripheral and optionally a fork; the
    /// channel number is the position, each register is resolved to its
    /// address, and the Task_Model records the channels.
    #[test]
    fn pm33_1_parses_ppi_channels_and_records_them_in_the_task_model() {
        let src = ppi_demo(
            "ppi compare_sets_pin (event = TIMER1[0x140], task = GPIOTE[0x030]);
             ppi edge_captures (event = GPIOTE[0x100], task = TIMER1[0x040], fork = TIMER1[0x008]);",
        );
        let parsed = parse_app(src.parse().unwrap()).unwrap();
        assert_eq!(parsed.decl.ppi.len(), 2);
        assert_eq!(parsed.decl.ppi[0], PpiDecl { name: "compare_sets_pin".into(), event: PpiRef { peripheral: "TIMER1".into(), offset: 0x140 }, task: PpiRef { peripheral: "GPIOTE".into(), offset: 0x030 }, fork: None });
        assert_eq!(parsed.decl.ppi[1].fork, Some(PpiRef { peripheral: "TIMER1".into(), offset: 0x008 }));
        // The name's span is recorded for diagnostics.
        let line = src.lines().position(|l| l.contains("ppi edge_captures")).unwrap() + 1;
        assert_eq!(parsed.spans.span_of(&ItemRef::Ppi("edge_captures".into())).start().line, line);

        let g = match crate::generate(&parsed.decl) {
            Ok(g) => g,
            Err(errs) => panic!("{}", errs.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")),
        };
        assert_eq!(g.ppi.len(), 2);
        assert_eq!((g.ppi[0].channel, g.ppi[1].channel), (0, 1));
        assert_eq!(g.ppi[0].event.address, 0x4000_9140);
        assert_eq!(g.ppi[0].task.address, 0x4000_6030);
        assert_eq!(g.ppi[1].fork.as_ref().unwrap().address, 0x4000_9008);

        let model = crate::model::task_model(&g).render();
        let compact: String = model.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(compact.contains(&format!("\"ppi\":{PPI_MODEL}")), "{model}");
    }

    /// A declaration without channels has no `ppi` key: the Task_Model of
    /// every existing Application is unchanged.
    #[test]
    fn pm33_1_no_channel_no_key() {
        let parsed = parse_app(DEMO.parse().unwrap()).unwrap();
        let g = crate::generate(&parsed.decl).unwrap();
        assert!(g.ppi.is_empty());
        assert!(!crate::model::task_model(&g).render().contains("\"ppi\""));
    }

    /// PM33.1: what a channel may name. Each rejection carries the channel.
    #[test]
    fn pm33_1_rejects_malformed_channels() {
        let item = |n: &str| format!("ppi {n}");
        // An unknown peripheral.
        assert_eq!(rules(&ppi_demo("ppi c (event = NOSUCH[0x140], task = GPIOTE[0x030]);")), vec![("R21.5".to_string(), item("c"))]);
        // An event offset in the task range, and a task offset in the event range.
        assert_eq!(rules(&ppi_demo("ppi c (event = TIMER1[0x040], task = GPIOTE[0x030]);")), vec![("PM33.1".to_string(), item("c"))]);
        assert_eq!(rules(&ppi_demo("ppi c (event = TIMER1[0x140], task = GPIOTE[0x100]);")), vec![("PM33.1".to_string(), item("c"))]);
        // A fork that is not a task register, and an offset that is not a multiple of 4.
        assert_eq!(rules(&ppi_demo("ppi c (event = TIMER1[0x140], task = GPIOTE[0x030], fork = TIMER1[0x504]);")), vec![("PM33.1".to_string(), item("c"))]);
        assert_eq!(rules(&ppi_demo("ppi c (event = TIMER1[0x142], task = GPIOTE[0x030]);")), vec![("PM33.1".to_string(), item("c"))]);
        // A name used twice.
        assert_eq!(
            rules(&ppi_demo("ppi c (event = TIMER1[0x140], task = GPIOTE[0x030]); ppi c (event = TIMER1[0x144], task = GPIOTE[0x034]);")),
            vec![("PR-28".to_string(), item("c"))]
        );
        // More channels than the Target has.
        let many: String = (0..21).map(|i| format!("ppi c{i} (event = TIMER1[0x140], task = GPIOTE[0x030]);")).collect();
        assert_eq!(rules(&ppi_demo(&many)), vec![("PM33.1".to_string(), "app".to_string())]);
        // A Target without a PPI.
        assert_eq!(rules(&format!("{DEMO}\nppi c (event = CMSDK_DUALTIMER[0x140], task = CMSDK_DUALTIMER[0x030]);")), vec![("PM33.1".to_string(), "app".to_string())]);
    }

    /// PM33.1: the surface errors of a channel are reported by the parser.
    #[test]
    fn pm33_1_surface_errors() {
        let bad = |c: &str| parse_app(ppi_demo(c).parse().unwrap()).err().map(|e| e.to_string()).unwrap_or_default();
        assert!(bad("ppi c (task = GPIOTE[0x030]);").contains("requires `event"));
        assert!(bad("ppi c (event = TIMER1[0x140]);").contains("requires `task"));
        assert!(bad("ppi c (event = TIMER1[0x140], event = TIMER1[0x144], task = GPIOTE[0x030]);").contains("given twice"));
        assert!(bad("ppi c (event = TIMER1[0x140], task = GPIOTE[0x030], shortcut = TIMER1[0x008]);").contains("unknown attribute `shortcut`"));
        assert!(!bad("ppi c (event = TIMER1[0x140], task = GPIOTE);").is_empty());
    }

    /// Until the Kernel writes the channels (docs/pacemaker tasks 3.2 and
    /// 3.3), nothing is emitted for a declaration that has one.
    #[test]
    fn pm33_3_no_output_for_channels_the_kernel_does_not_write_yet() {
        let src = ppi_demo("ppi c (event = TIMER1[0x140], task = GPIOTE[0x030]);");
        let parsed = parse_app(src.parse().unwrap()).unwrap();
        assert!(crate::generate(&parsed.decl).is_ok());
        let Err(errs) = crate::outputs(&parsed.decl, "sys") else { panic!("outputs were emitted") };
        assert_eq!(errs.iter().map(|e| (e.rule.as_str(), e.item.to_string())).collect::<Vec<_>>(), vec![("PM33.3", "ppi c".to_string())]);
        assert!(!crate::PPI_KERNEL_SUPPORT, "when the Kernel writes the channels, this test goes and `outputs` emits them");
    }

    /// PM33.2 (docs/pacemaker): a channel's peripherals are all owned by
    /// one Partition. Negative fixtures: a channel across two Partitions
    /// (by its task, and by its fork), a Kernel-owned peripheral, and a
    /// peripheral no Partition owns.
    #[test]
    fn pm33_2_a_channel_stays_inside_one_partition() {
        let item = |n: &str| format!("ppi {n}");
        // part0 owns TIMER1 and part1 owns GPIOTE.
        let split = |channels: &str| {
            let src = DEMO
                .replace("target = qemu_mps2_an386", "target = nrf52840")
                .replace("peripherals = [];", "peripherals = [TIMER1];")
                .replace("task log (periodic", "peripherals = [GPIOTE];\n            task log (periodic");
            assert_eq!(src.matches("peripherals =").count(), 2);
            format!("{src}\n{channels}")
        };
        assert_eq!(rules(&split("ppi c (event = TIMER1[0x140], task = GPIOTE[0x030]);")), vec![("PM33.2".to_string(), item("c"))]);
        assert_eq!(rules(&split("ppi c (event = TIMER1[0x140], task = TIMER1[0x040], fork = GPIOTE[0x030]);")), vec![("PM33.2".to_string(), item("c"))]);
        // Inside either Partition the same registers are accepted.
        assert_eq!(rules(&split("ppi c (event = TIMER1[0x140], task = TIMER1[0x040]); ppi d (event = GPIOTE[0x100], task = GPIOTE[0x030]);")), vec![]);
        // A Kernel-owned peripheral: the time base, and the PPI itself.
        assert_eq!(rules(&ppi_demo("ppi c (event = TIMER0[0x140], task = GPIOTE[0x030]);")), vec![("PR-29".to_string(), item("c"))]);
        assert_eq!(rules(&ppi_demo("ppi c (event = TIMER1[0x140], task = PPI[0x000]);")), vec![("PR-29".to_string(), item("c"))]);
        // A peripheral of the Target that no Partition lists.
        assert_eq!(rules(&ppi_demo("ppi c (event = TIMER2[0x140], task = GPIOTE[0x030]);")), vec![("PM33.2".to_string(), item("c"))]);
        // The message names both Partitions.
        let parsed = parse_app(split("ppi c (event = TIMER1[0x140], task = GPIOTE[0x030]);").parse().unwrap()).unwrap();
        let text = crate::generate(&parsed.decl).unwrap_err()[0].to_string();
        assert!(text.contains("`TIMER1` of Partition `part0`") && text.contains("`GPIOTE` of Partition `part1`"), "{text}");
    }

    /// PM33.2: the derived channel records the owner of each peripheral,
    /// which is what the proven acceptance check decides on.
    #[test]
    fn pm33_2_the_owner_of_each_register_is_derived() {
        let parsed = parse_app(ppi_demo("ppi c (event = TIMER1[0x140], task = GPIOTE[0x030], fork = TIMER1[0x008]);").parse().unwrap()).unwrap();
        let g = crate::generate(&parsed.decl).unwrap();
        assert_eq!((g.ppi[0].event.owner, g.ppi[0].task.owner, g.ppi[0].fork.as_ref().unwrap().owner), (Some(0), Some(0), Some(0)));
        let (.., ppi, limit) = crate::accept_v::image(&g);
        assert_eq!(limit, 20);
        assert_eq!((ppi[0].channel, ppi[0].event_offset, ppi[0].task_offset, ppi[0].fork, ppi[0].fork_offset), (0, 0x140, 0x030, true, 0x008));
        assert_eq!((ppi[0].event_owner, ppi[0].task_owner, ppi[0].fork_owner), (0, 0, 0));
    }

    /// PM33.4: the channels are under the configuration checksum. Without
    /// channels the image is what it was; with channels it ends in their
    /// table, and a change of any register, of the fork, or of the order
    /// changes the checksum.
    #[test]
    fn pm33_4_the_channels_are_under_the_configuration_checksum() {
        let generate = |channels: &str| crate::generate(&parse_app(ppi_demo(channels).parse().unwrap()).unwrap().decl).unwrap();
        let none = generate("");
        assert!(crate::derive::ppi_bytes(&none).is_empty());
        let a = "ppi a (event = TIMER1[0x140], task = GPIOTE[0x030]);";
        let b = "ppi b (event = GPIOTE[0x100], task = TIMER1[0x040], fork = TIMER1[0x008]);";
        let both = generate(&format!("{a} {b}"));
        let image = crate::derive::config_bytes(&both);
        let base = crate::derive::config_bytes(&none);
        #[rustfmt::skip]
        let table = [
            2,
            0, 0x40, 0x91, 0x00, 0x40, 0x30, 0x60, 0x00, 0x40, 0, 0, 0, 0,
            1, 0x00, 0x61, 0x00, 0x40, 0x40, 0x90, 0x00, 0x40, 0x08, 0x90, 0x00, 0x40,
        ];
        assert_eq!(image[..base.len()], base[..]);
        assert_eq!(image[base.len()..], table);
        assert_eq!(both.checksum, crate::derive::crc32(&image));
        let mut sums = vec![none.checksum, both.checksum];
        for other in [
            a.to_string(),
            format!("{b} {a}"),
            format!("{a} {}", b.replace("0x008", "0x00c")),
            format!("{a} {}", b.replace(", fork = TIMER1[0x008]", "")),
            format!("{} {b}", a.replace("0x140", "0x144")),
            format!("{} {b}", a.replace("0x030", "0x034")),
        ] {
            sums.push(generate(&other).checksum);
        }
        let distinct: std::collections::BTreeSet<u32> = sums.iter().copied().collect();
        assert_eq!(distinct.len(), sums.len(), "{sums:08x?}");
    }

    /// PM33.4: the emitted configuration carries the channel table, and a
    /// configuration without channels has no such item.
    #[test]
    fn pm33_4_the_channel_table_is_emitted() {
        let generate = |channels: &str| crate::generate(&parse_app(ppi_demo(channels).parse().unwrap()).unwrap().decl).unwrap();
        let g = generate("ppi a (event = TIMER1[0x140], task = GPIOTE[0x030]); ppi b (event = GPIOTE[0x100], task = TIMER1[0x040], fork = TIMER1[0x008]);");
        let source = crate::emit::system_source(&g);
        assert!(
            source.contains("pub static RSK_PPI: [[u32; 3]; 2] = [[0x40009140, 0x40006030, 0x00000000], [0x40006100, 0x40009040, 0x40009008]];"),
            "{}",
            source.lines().filter(|l| l.contains("PPI")).collect::<Vec<_>>().join("\n")
        );
        assert!(source.contains(&format!("pub const CONFIG_CHECKSUM: u32 = 0x{:08x};", g.checksum)));
        assert!(!crate::emit::system_source(&generate("")).contains("RSK_PPI"));
    }

    /// PM33.6 (docs/pacemaker): TIMER3 of the nRF52840 is a peripheral a
    /// Partition may own and a Task may bind: its window is the Partition's,
    /// its interrupt line 26 releases the Task, and channels may name it.
    #[test]
    fn pm33_6_timer3_can_be_owned_bound_and_connected() {
        let src = DEMO
            .replace("target = qemu_mps2_an386", "target = nrf52840")
            .replace("peripherals = [];", "peripherals = [TIMER3, GPIOTE];")
            .replace(
                "task sensor (periodic, period = 20.ms(), deadline = 20.ms(), budget = 19.ms(), priority = 5, resources = [state]);",
                "task sensor (sporadic, binds = TIMER3, mit = 20.ms(), mit_policy = defer, deadline = 20.ms(), budget = 9.ms(), priority = 5, resources = [state]);",
            );
        assert!(src.contains("binds = TIMER3"));
        let src = format!("{src}\nppi pulse_end (event = TIMER3[0x150], task = GPIOTE[0x060]);");
        let parsed = parse_app(src.parse().unwrap()).unwrap();
        let g = match crate::generate(&parsed.decl) {
            Ok(g) => g,
            Err(errs) => panic!("{}", errs.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")),
        };
        let per = g.target.peripheral("TIMER3").unwrap();
        assert_eq!((per.window_base, per.window_size, per.irq, per.kernel_owned, per.easy_dma), (0x4001_A000, 0x1000, Some(26), false, false));
        assert_eq!(g.ppi[0].event.address, 0x4001_A150);
        let model = crate::model::task_model(&g).render();
        let compact: String = model.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(compact.contains("\"TIMER3\""), "{model}");
        assert!(compact.contains("\"irq\":26"), "{model}");
        // The Task_Model of the declaration without the channel is the
        // Config_Checker's fixture: its test recomputes every derived value
        // of it (`the_timer3_task_model_recomputes_without_discrepancy`).
        let plain = src.replace("ppi pulse_end (event = TIMER3[0x150], task = GPIOTE[0x060]);", "");
        let fixture = crate::model::task_model(&crate::generate(&parse_app(plain.parse().unwrap()).unwrap().decl).unwrap()).render();
        assert_eq!(fixture, include_str!("../../rsk-confcheck/fixtures/pm33_6_timer3_task_model.json"), "regenerate the Config_Checker's fixture");
        // Two Partitions cannot both own it, and the QEMU Target has none.
        let twice = src.replace("task log (periodic", "peripherals = [TIMER3];\n            task log (periodic");
        assert!(rules(&twice).iter().any(|(r, i)| r == "PR-28" && i.contains("TIMER3")), "{:?}", rules(&twice));
        assert!(rules(&DEMO.replace("peripherals = [];", "peripherals = [TIMER3];")).iter().any(|(r, _)| r == "R21.5"));
    }

    /// R3.1, R21.4: a Generator diagnostic is reported at the source span of
    /// the offending item, with the rule it violates: a Priority beyond the
    /// hardware levels at the Task's `priority` field, a response outside its
    /// set at the Partition's field, each on the line where it is written.
    #[test]
    fn r3_1_r21_4_a_violation_is_reported_at_the_span_of_the_offending_item() {
        let src = DEMO.replace("priority = 2", "priority = 9").replace(
            "fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY, mit_threshold = 10) {\n            task log",
            "fault = RECORD_ONLY, overrun = END_JOB, deadline_miss = RECORD_ONLY, mit_threshold = 10) {\n            task log",
        );
        assert_ne!(src, DEMO);
        let line_of = |needle: &str| src.lines().position(|l| l.contains(needle)).expect(needle) + 1;
        let parsed = parse_app(src.parse().unwrap()).unwrap();
        let errs = crate::generate(&parsed.decl).unwrap_err();
        let resolved = parsed.spans.resolve(&errs);
        assert_eq!(resolved.len(), errs.len());
        let at = |rule: &str| resolved.iter().find(|e| e.message.starts_with(&format!("[{rule}]"))).unwrap_or_else(|| panic!("{rule}: {resolved:?}"));
        let prio = at("PR-33");
        assert_eq!(prio.span.start().line, line_of("priority = 9"), "{}", prio.message);
        let fault = at("R15.7");
        assert_eq!(fault.span.start().line, line_of("fault = RECORD_ONLY"), "{}", fault.message);
        // The two are different places of the declaration.
        assert_ne!(prio.span.start().line, fault.span.start().line);
    }

    /// R21.2: a time quantity without an explicit unit is rejected.
    #[test]
    fn a_time_without_unit_is_rejected() {
        let src = DEMO.replace("period = 10.ms()", "period = 10");
        let e = parse_app(src.parse().unwrap()).err().expect("rejected");
        assert!(e.to_string().contains("explicit unit"), "{e}");
    }

    /// R21.1, R21.2, R23.1, R23.4: sporadic Tasks with interrupt and signal
    /// Release_Sources, MIT policy and FPU use are accepted; the Budget is
    /// converted to cycles, the Ceiling and the timer-slot count are derived.
    #[test]
    fn parses_sporadic_tasks_and_signals() {
        let src = r#"
            target = nrf52840, operating_duration = 1.h(), safe_state = app::safe;
            partition nav (level = A, code = 64.KiB(), ram = 16.KiB(), stack = 4.KiB(), fault = RESTART_PARTITION,
                           overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY, mit_threshold = 10) {
                peripherals = [GPIOTE];
                resource state: nav::State = nav::State::new();
                signal tick;
                task imu (sporadic, binds = GPIOTE, mit = 1.ms(), mit_policy = defer, deadline = 1.ms(), budget = 40.us(), priority = 6, fpu, resources = [state]);
                task filter (sporadic, signal = tick, mit = 5.ms(), deadline = 5.ms(), budget = 100.us(), priority = 4, resources = [state]);
                task control (periodic, period = 10.ms(), deadline = 10.ms(), budget = 120.us(), priority = 5, resources = [state], raises = [tick]);
            }
        "#;
        let parsed = parse_app(src.parse().unwrap()).unwrap();
        let g = crate::generate(&parsed.decl).unwrap();
        assert_eq!(g.tasks[0].kind, crate::derive::Kind::Sporadic);
        assert_eq!(g.tasks[0].budget_cycles, 2560);
        assert_eq!(g.resources[0].ceiling, 6);
        assert_eq!(g.slot_count, 3 * 3 + 2);
    }

    /// R46.4: a Resource invariant is declared with the Resource and reaches
    /// the App_Prover as an open logic function and each Task's precondition.
    #[test]
    fn parses_a_resource_invariant_for_the_app_prover_r46_4() {
        let src = DEMO.replace(
            "resource state: u32 = 0;",
            "resource state: u32 = 0 invariant |v: u32| v@ <= 1 && v@ >= 0;",
        );
        let parsed = parse_app(src.parse().unwrap()).unwrap();
        let inv = parsed.decl.partitions[0].resources[0].invariant.clone().expect("invariant");
        assert_eq!(inv.replace(' ', ""), "|v:u32|v@<=1&&v@>=0");
        let g = crate::generate(&parsed.decl).unwrap();
        let part0 = crate::emit::partition_source(&g, 0);
        assert!(part0.contains("#[cfg(creusot)]\n#[allow(unused_imports, unused_variables)]\npub mod rsk_spec {"), "{part0}");
        assert!(part0.contains("pub fn state_inv(v: u32) -> bool { pearlite! { v @ <= 1 && v @ >= 0 } }"), "{part0}");
        assert!(part0.contains("proof_assert!(state_inv(init));"), "{part0}");
        assert!(part0.contains("(forall<x: u32> (*cx).state@[x] == state_inv(x))"), "{part0}");
        // Without a declared invariant, the trivial one.
        let g = crate::generate(&parse_app(DEMO.parse().unwrap()).unwrap().decl).unwrap();
        assert!(crate::emit::partition_source(&g, 0).contains("pub fn state_inv(_v: u32) -> bool { pearlite! { true } }"));
        // A malformed invariant is a declaration error.
        let bad = DEMO.replace("resource state: u32 = 0;", "resource state: u32 = 0 invariant |v: u32|;");
        assert!(parse_app(bad.parse().unwrap()).is_err());
    }

    #[test]
    fn finds_the_invocation_in_a_source_file() {
        let src = format!("#![no_std]\nrsk::app! {{ {DEMO} }}\nfn report_and_exit() -> ! {{ loop {{}} }}\n");
        assert!(parse_source(&src).is_ok());
        assert!(parse_source("fn main() {}").is_err());
    }
}
