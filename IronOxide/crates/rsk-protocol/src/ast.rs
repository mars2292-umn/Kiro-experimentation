//! The global and local types.

/// One message of a global type: its label, its payload type as written
/// (absent for a signal with no payload), and the two roles.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Message {
    pub label: String,
    pub payload: Option<String>,
    pub from: String,
    pub to: String,
}

/// A global Protocol (R38.2: sequencing, labelled choice, recursion).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Global {
    End,
    /// A message, then the rest.
    Msg(Message, Box<Global>),
    /// A labelled choice made by `at`: every branch starts with a message
    /// from `at`.
    Choice { at: String, branches: Vec<Global> },
    Rec { name: String, body: Box<Global> },
    Continue(String),
}

/// One branch of a local choice: the label, its payload type, the rest.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Branch {
    pub label: String,
    pub payload: Option<String>,
    pub cont: Local,
}

/// A role's local type (R38.3): what the typestate API of task 21.3 is
/// generated from.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Local {
    End,
    /// An internal choice: the role sends one of the labels to `to`.
    Send { to: String, branches: Vec<Branch> },
    /// An external choice: the role receives one of the labels from `from`.
    Recv { from: String, branches: Vec<Branch> },
    Rec { name: String, body: Box<Local> },
    Continue(String),
}

impl Global {
    /// The roles the type names, in order of first appearance.
    pub fn roles(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_roles(&mut out);
        out
    }

    fn collect_roles(&self, out: &mut Vec<String>) {
        match self {
            Global::End | Global::Continue(_) => {}
            Global::Msg(m, rest) => {
                for r in [&m.from, &m.to] {
                    if !out.contains(r) {
                        out.push(r.clone());
                    }
                }
                rest.collect_roles(out);
            }
            Global::Choice { at, branches } => {
                if !out.contains(at) {
                    out.push(at.clone());
                }
                for b in branches {
                    b.collect_roles(out);
                }
            }
            Global::Rec { body, .. } => body.collect_roles(out),
        }
    }

    /// The type rendered in the checker's syntax (one line).
    pub fn render(&self) -> String {
        match self {
            Global::End => "end".to_string(),
            Global::Msg(m, rest) => format!("{}({}): {} -> {}; {}", m.label, m.payload.clone().unwrap_or_default(), m.from, m.to, rest.render()),
            Global::Choice { at, branches } => {
                let bs: Vec<String> = branches.iter().map(|b| format!("{{ {} }}", b.render())).collect();
                format!("choice at {at} {}", bs.join(" or "))
            }
            Global::Rec { name, body } => format!("rec {name} {{ {} }}", body.render()),
            Global::Continue(n) => format!("continue {n}"),
        }
    }
}

impl Local {
    /// The local type rendered as `!to{label(payload). ..}` and
    /// `?from{..}`, the usual notation.
    pub fn render(&self) -> String {
        fn branches(bs: &[Branch]) -> String {
            bs.iter().map(|b| format!("{}({}).{}", b.label, b.payload.clone().unwrap_or_default(), b.cont.render())).collect::<Vec<_>>().join(", ")
        }
        match self {
            Local::End => "end".to_string(),
            Local::Send { to, branches: bs } => format!("{to}!{{{}}}", branches(bs)),
            Local::Recv { from, branches: bs } => format!("{from}?{{{}}}", branches(bs)),
            Local::Rec { name, body } => format!("rec {name}.{}", body.render()),
            Local::Continue(n) => n.clone(),
        }
    }

    /// Whether the type has any send or receive.
    pub fn is_silent(&self) -> bool {
        match self {
            Local::End | Local::Continue(_) => true,
            Local::Send { .. } | Local::Recv { .. } => false,
            Local::Rec { body, .. } => body.is_silent(),
        }
    }
}
