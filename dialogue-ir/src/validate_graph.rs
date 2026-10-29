//! The two checks that walk the graph (ADR 28.10.26f D2): rule 6, **no
//! cycle made only of edges without `when`**, and the unreachable-checkpoint
//! warning. Both run after rules 1 and 2, so ids are unique and every
//! non-outcome `to` names a checkpoint.
//!
//! The core crosses a `when`-less edge alone, with no reply between, so a
//! cycle of them would loop with no turn, whatever their predicates say. Any
//! other cycle is valid. The search is a three-colour depth-first walk over
//! the `when`-less edges between checkpoints: at most 64 nodes, so the
//! recursion is bounded.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::ir::DialogueIr;
use crate::path::edge;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Colour {
    White,
    Grey,
    Black,
}

/// One `when`-less edge: the checkpoint it leads to and its path.
type Arc = (usize, String);

/// The path of an edge on a `when`-less cycle, if there is one.
pub(crate) fn when_less_cycle(ir: &DialogueIr) -> Option<String> {
    let at: HashMap<&str, usize> = ir.checkpoints.iter().enumerate().map(|(i, c)| (c.id.as_str(), i)).collect();
    let arcs: Vec<Vec<Arc>> = ir
        .checkpoints
        .iter()
        .enumerate()
        .map(|(i, checkpoint)| {
            let when_less = checkpoint.edges.iter().enumerate().filter(|(_, e)| e.when.is_none());
            when_less.filter_map(|(j, e)| Some((*at.get(e.to.as_str())?, edge(i, j)))).collect()
        })
        .collect();
    let mut colour = vec![Colour::White; arcs.len()];
    (0..arcs.len()).find_map(|start| if colour[start] == Colour::White { visit(start, &arcs, &mut colour) } else { None })
}

/// Grey on the stack, black when done; an arc into a grey node closes a cycle.
fn visit(node: usize, arcs: &[Vec<Arc>], colour: &mut [Colour]) -> Option<String> {
    colour[node] = Colour::Grey;
    for (to, path) in &arcs[node] {
        match colour[*to] {
            Colour::Grey => return Some(path.clone()),
            Colour::White => {
                if let Some(found) = visit(*to, arcs, colour) {
                    return Some(found);
                }
            }
            Colour::Black => {}
        }
    }
    colour[node] = Colour::Black;
    None
}

/// The indices of the checkpoints no path of edges reaches from `start`,
/// whatever their conditions say.
pub(crate) fn unreachable(ir: &DialogueIr) -> Vec<usize> {
    let mut seen = HashSet::from([ir.start.as_str()]);
    let mut queue = VecDeque::from([ir.start.as_str()]);
    while let Some(id) = queue.pop_front() {
        let Some(checkpoint) = ir.checkpoint(id) else { continue };
        for edge in &checkpoint.edges {
            if ir.checkpoint(&edge.to).is_some() && seen.insert(edge.to.as_str()) {
                queue.push_back(edge.to.as_str());
            }
        }
    }
    ir.checkpoints.iter().enumerate().filter(|(_, c)| !seen.contains(c.id.as_str())).map(|(i, _)| i).collect()
}
