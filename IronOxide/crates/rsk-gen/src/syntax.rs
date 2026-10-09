//! The `rsk::app!` surface syntax (design.md example; R21.1), parsed with
//! `syn` into a [`Declaration`] plus a span map, so that the front end can
//! report every Generator diagnostic at the source span of the offending
//! item (R21.4, R3.1). The CLI uses the same parser on the declaration
//! crate's source (R24.3), so the macro and the CLI cannot diverge.
//!
//! ```text
//! rsk::app! {
//!     target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 200.ms(),
//!     safe_state = crate::report_and_exit, log_capacity = 32;
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
            None => err(self.owner, format!("{} requires `{name} = ...` (R21.3)", self.what)),
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
            return err(a.span, format!("unknown attribute `{}` of {}", a.name, self.what));
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
        h.finish()?;

        let mut partitions = Vec::new();
        while !input.is_empty() {
            let kw: Ident = input.parse()?;
            if kw != "partition" {
                return err(kw.span(), "expected `partition`");
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
            let mut tasks = Vec::new();
            let mut signals = Vec::new();
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
                        body.parse::<Token![;]>()?;
                        resources.push(ResourceDecl {
                            name: rname.to_string(),
                            ty: tokens_to_string(&ty),
                            init: tokens_to_string(&init),
                            placement: ResourcePlacement::Core(0),
                        });
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
                            let source = match (binds, signal) {
                                (Some(b), None) => Source::Interrupt(ident(b)?.0),
                                (None, Some(s)) => Source::Signal(ident(s)?.0),
                                (Some(b), Some(_)) => return err(b.span, "a task has exactly one Release_Source: `binds` or `signal` (PR-11, R22.4)"),
                                (None, None) => return err(tname.span(), "a sporadic task declares its Release_Source: `binds = PERIPHERAL` or `signal = NAME` (PR-11, R22.4)"),
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
                        });
                    }
                    other => return err(kw.span(), format!("unknown item `{other}` in a partition (peripherals, resource, task, signal)")),
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
                partitions,
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

    #[test]
    fn a_time_without_unit_is_rejected() {
        let src = DEMO.replace("period = 10.ms()", "period = 10");
        let e = parse_app(src.parse().unwrap()).err().expect("rejected");
        assert!(e.to_string().contains("explicit unit"), "{e}");
    }

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

    #[test]
    fn finds_the_invocation_in_a_source_file() {
        let src = format!("#![no_std]\nrsk::app! {{ {DEMO} }}\nfn report_and_exit() -> ! {{ loop {{}} }}\n");
        assert!(parse_source(&src).is_ok());
        assert!(parse_source("fn main() {}").is_err());
    }
}
