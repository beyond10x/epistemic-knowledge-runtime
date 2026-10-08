//! Bounded YAML observation and alias accounting shared by document readers before decoding.
//!
//! The pinned loader's scan is quadratic in flow nesting depth, so a depth refusal that runs after
//! the full load still pays for the load. [`load`] stops the parser at the first container nested
//! deeper than the reader's depth: the document's event tape ends at that container's start and
//! carries a recursion limit error, and no later event or document is read.
//!
//! A reader may walk the bounded tape with its own budget, or use [`expand`], which totals
//! expanded nodes, text bytes and depth without re-walking an alias:
//! each anchored node's expanded size is recorded once, when it closes, and an alias adds it.

use std::collections::{BTreeMap, BTreeSet};

use serde_yaml_ng::observation::{Document, Documents, Event};
use serde_yaml_ng::Error;

/// Begin loading `text`, stopping the parser at the first container nested deeper than `depth`.
pub fn load(text: &str, depth: usize) -> Result<Documents<'_>, Error> {
    Documents::from_str_within_depth(text, depth)
}

/// The inclusive limits [`expand`] holds a document to.
#[derive(Clone, Copy, Debug)]
pub struct Expansion {
    /// Nested containers, aliases expanded in place; root is one.
    pub depth: usize,
    /// Scalars, keys and containers, each alias counted as the nodes it repeats.
    pub nodes: u64,
    /// Decoded scalar and key bytes, each alias counted as the text it repeats.
    pub text_bytes: u64,
}

/// What [`expand`] refused.
#[derive(Debug)]
pub enum Past {
    /// A container, or an alias's expansion, nests deeper than [`Expansion::depth`].
    Depth,
    /// The expanded nodes pass [`Expansion::nodes`].
    Nodes(u64),
    /// The expanded text passes [`Expansion::text_bytes`].
    Text(u64),
    /// An alias inside the node it names, which expands without end.
    Recursive,
    /// An event the observation facade could not read.
    Malformed(Error),
}

/// Running totals across every document of one input.
#[derive(Default)]
pub struct Tally {
    nodes: u64,
    text_bytes: u64,
}

/// One node's expanded size: its nodes, its text and how many containers deep it reaches.
#[derive(Clone, Copy, Default)]
struct Size {
    nodes: u64,
    text_bytes: u64,
    height: usize,
}

impl Size {
    fn add(&mut self, other: Self) {
        self.nodes = self.nodes.saturating_add(other.nodes);
        self.text_bytes = self.text_bytes.saturating_add(other.text_bytes);
        self.height = self.height.max(other.height);
    }
}

/// Totals `document`'s expansion into `tally` and refuses the first limit it passes, in one pass
/// over the event tape and a second over its aliases: an alias is never re-walked. A tape the
/// loader cut short at its depth bound is refused as [`Past::Depth`] at the container it stopped on.
pub fn expand(document: &Document<'_>, limits: Expansion, tally: &mut Tally) -> Result<(), Past> {
    let event = |index: usize| document.event(index).map_err(Past::Malformed);
    // Only an aliased node's size is kept, so the sizes held are bounded by the aliases written.
    let mut targets = BTreeSet::new();
    for index in 0..document.event_count() {
        if let Some(Event::Alias { target }) = event(index)? {
            targets.insert(target);
        }
    }
    let mut sizes: BTreeMap<usize, Size> = BTreeMap::new();
    // Each open container: where it starts and the size of what it holds so far.
    let mut open: Vec<(usize, Size)> = Vec::new();
    for index in 0..document.event_count() {
        let closed = match event(index)? {
            Some(Event::Scalar(scalar)) => {
                let text_bytes = scalar.text().map_err(Past::Malformed)?.len() as u64;
                Some((
                    index,
                    Size {
                        nodes: 1,
                        text_bytes,
                        height: 0,
                    },
                ))
            }
            Some(Event::Empty) => Some((
                index,
                Size {
                    nodes: 1,
                    ..Size::default()
                },
            )),
            Some(Event::SequenceStart(_) | Event::MappingStart(_)) => {
                charge(
                    tally,
                    limits,
                    Size {
                        nodes: 1,
                        ..Size::default()
                    },
                )?;
                open.push((
                    index,
                    Size {
                        nodes: 1,
                        ..Size::default()
                    },
                ));
                if open.len() > limits.depth {
                    return Err(Past::Depth);
                }
                None
            }
            Some(Event::SequenceEnd | Event::MappingEnd) => {
                let Some((start, mut size)) = open.pop() else {
                    return Ok(());
                };
                size.height += 1;
                if targets.contains(&start) {
                    sizes.insert(start, size);
                }
                add_to_parent(&mut open, size);
                None
            }
            Some(Event::Alias { target }) => {
                let size = *sizes.get(&target).ok_or(Past::Recursive)?;
                if open.len().saturating_add(size.height) > limits.depth {
                    return Err(Past::Depth);
                }
                charge(tally, limits, size)?;
                add_to_parent(&mut open, size);
                None
            }
            None => None,
        };
        if let Some((start, size)) = closed {
            charge(tally, limits, size)?;
            if targets.contains(&start) {
                sizes.insert(start, size);
            }
            add_to_parent(&mut open, size);
        }
    }
    Ok(())
}

/// Adds a closed node to the container holding it, if any.
fn add_to_parent(open: &mut [(usize, Size)], size: Size) {
    if let Some((_, parent)) = open.last_mut() {
        parent.add(size);
    }
}

/// Adds `size` to the running totals, refusing the first limit it passes.
fn charge(tally: &mut Tally, limits: Expansion, size: Size) -> Result<(), Past> {
    tally.nodes = tally.nodes.saturating_add(size.nodes);
    tally.text_bytes = tally.text_bytes.saturating_add(size.text_bytes);
    if tally.nodes > limits.nodes {
        return Err(Past::Nodes(tally.nodes));
    }
    if tally.text_bytes > limits.text_bytes {
        return Err(Past::Text(tally.text_bytes));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: Expansion = Expansion {
        depth: 4,
        nodes: 20,
        text_bytes: 12,
    };

    fn walk(text: &str) -> Result<Tally, Past> {
        let mut documents = load(text, LIMITS.depth).unwrap();
        let mut tally = Tally::default();
        while let Some(document) = documents.next_document() {
            expand(&document, LIMITS, &mut tally)?;
            if document.check().is_err() {
                break;
            }
        }
        Ok(tally)
    }

    #[test]
    fn an_alias_counts_as_what_it_repeats_and_is_never_re_walked() {
        // `a` is a sequence of three one-byte scalars: 4 nodes, 3 bytes. Two aliases repeat it.
        let tally = walk("{k: &a [x, y, z], l: *a, m: *a}").unwrap();
        // Root map 1, keys 3 (3 bytes), the anchored sequence 4 (3 bytes), aliases 2 x 4 (2 x 3).
        assert_eq!((tally.nodes, tally.text_bytes), (16, 12));
        // One more byte of text anywhere passes the text limit.
        assert!(matches!(
            walk("{k: &a [x, y, z], l: *a, m: *a, n: q}"),
            Err(Past::Nodes(_) | Past::Text(_))
        ));
    }

    #[test]
    fn a_nested_alias_bomb_is_refused_by_count_without_expanding() {
        let mut text = String::from("{a: &a [x, x]");
        for level in 1..=40 {
            let previous = level - 1;
            let name = format!("a{level}");
            let alias = if previous == 0 {
                "*a".to_owned()
            } else {
                format!("*a{previous}")
            };
            text.push_str(&format!(", {name}: &{name} [{alias}, {alias}]"));
        }
        text.push('}');
        // 2^40 nodes if expanded; the walk refuses on the totals alone.
        assert!(matches!(walk(&text), Err(Past::Nodes(_) | Past::Text(_))));
    }

    #[test]
    fn depth_is_counted_at_the_cut_and_through_an_alias() {
        assert!(walk("[[[[x]]]]").is_ok());
        assert!(matches!(walk("[[[[[x]]]]]"), Err(Past::Depth)));
        // The anchored node is 2 deep; repeated 3 containers down it reaches 5.
        assert!(matches!(walk("[&a [[x]], [[*a]]]"), Err(Past::Depth)));
        assert!(walk("[&a [[x]], [*a]]").is_ok());
    }

    #[test]
    fn an_alias_inside_its_own_anchor_is_recursive() {
        assert!(matches!(walk("&a [*a]"), Err(Past::Recursive)));
    }
}
