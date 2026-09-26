//! Generic transaction storage for undo/redo on a CRDT graph.
//!
//! The 2P2P-Graph CRDT is append-only: removing a vertex or edge adds
//! to `V_R` / `E_R` rather than mutating in place. That makes undo a
//! matter of *recording compensating operations* (the inverse of what
//! was just applied) and replaying them on demand — the CRDT state
//! grows monotonically, but the user sees the active set flip back to
//! its pre-edit shape.
//!
//! This module provides the **storage layer** for that pattern. It is
//! op-agnostic: the caller picks whatever `Op` type makes sense
//! (`UpdateOperation<...>`, a custom enum like an RDF triple action,
//! etc.) and is responsible for *applying* ops to its concrete graph.
//! The stack only owns the bookkeeping — undo / redo / drop / clear /
//! history-limit eviction.
//!
//! Specialising layers (e.g. `crdf`) wrap this stack and supply
//! - a diff routine that produces a list of compensating ops,
//! - an apply routine that executes ops against the graph,
//! to get a transactional undo/redo API tailored to their data model.

/// Default cap on the number of transactions retained per stack. Old
/// entries are dropped from the bottom so long sessions don't grow
/// unbounded.
pub const DEFAULT_TRANSACTION_HISTORY_LIMIT: usize = 256;

/// A logical edit recorded for undo/redo.
///
/// `forward` is the sequence of ops the edit applied to the graph;
/// `inverse` is what the caller should apply to roll it back. Both are
/// kept so the stack can support symmetric undo *and* redo from a
/// single stored value.
#[derive(Clone, Debug)]
pub struct GraphTransaction<Op> {
    label: String,
    forward: Vec<Op>,
    inverse: Vec<Op>,
}

impl<Op> GraphTransaction<Op> {
    /// Construct a transaction. An empty (`forward.is_empty() &&
    /// inverse.is_empty()`) transaction is allowed at this layer —
    /// [`TransactionStack::push`] rejects it.
    pub fn new(label: impl Into<String>, forward: Vec<Op>, inverse: Vec<Op>) -> Self {
        Self {
            label: label.into(),
            forward,
            inverse,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn forward(&self) -> &[Op] {
        &self.forward
    }

    pub fn inverse(&self) -> &[Op] {
        &self.inverse
    }

    /// Move out the inverse ops. Use this when the caller applies them
    /// to the graph during undo and wants to avoid cloning.
    pub fn take_inverse(self) -> (String, Vec<Op>, Vec<Op>) {
        (self.label, self.forward, self.inverse)
    }

    pub fn is_empty(&self) -> bool {
        self.forward.is_empty() && self.inverse.is_empty()
    }
}

/// Op-agnostic undo/redo stack.
///
/// The stack stores `GraphTransaction<Op>` values; it does **not**
/// apply them. After [`pop_undo`](Self::pop_undo) the caller is
/// expected to execute the transaction's inverse against the graph and
/// then move the transaction onto the redo stack via
/// [`push_to_redo`](Self::push_to_redo). The two operations are split
/// so the caller can short-circuit on errors without leaving the stack
/// in a torn state.
pub struct TransactionStack<Op> {
    undo: Vec<GraphTransaction<Op>>,
    redo: Vec<GraphTransaction<Op>>,
    limit: usize,
}

impl<Op> Default for TransactionStack<Op> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Op> TransactionStack<Op> {
    pub fn new() -> Self {
        Self::with_limit(DEFAULT_TRANSACTION_HISTORY_LIMIT)
    }

    /// Construct a stack that retains at most `limit` undo transactions.
    /// `limit == 0` means **no limit**: every transaction is kept. (This
    /// said "disables history" until 2026-09-27, which the code never did;
    /// callers rely on the unbounded behaviour.)
    pub fn with_limit(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit,
        }
    }

    /// Push a freshly-committed transaction onto the undo stack and
    /// clear the redo stack (standard linear history). Returns `false`
    /// when the transaction was empty (and therefore not pushed).
    pub fn push(&mut self, tx: GraphTransaction<Op>) -> bool {
        if tx.is_empty() {
            return false;
        }
        self.undo.push(tx);
        self.redo.clear();
        self.trim();
        true
    }

    /// Pop the most recent undo transaction so the caller can execute
    /// its inverse. The caller is then responsible for calling
    /// [`push_to_redo`](Self::push_to_redo) to make the transaction
    /// available for redo.
    pub fn pop_undo(&mut self) -> Option<GraphTransaction<Op>> {
        self.undo.pop()
    }

    /// Symmetric of [`pop_undo`](Self::pop_undo): pop the most recent
    /// redo transaction so the caller can execute its `forward` ops.
    pub fn pop_redo(&mut self) -> Option<GraphTransaction<Op>> {
        self.redo.pop()
    }

    /// Move a previously-popped transaction onto the redo stack. Used
    /// after an undo to make the action redoable.
    pub fn push_to_redo(&mut self, tx: GraphTransaction<Op>) {
        self.redo.push(tx);
    }

    /// Move a previously-popped transaction back onto the undo stack.
    /// Used after a redo to make the action undoable again, or to
    /// re-instate a transaction whose execution was aborted.
    pub fn push_to_undo(&mut self, tx: GraphTransaction<Op>) {
        self.undo.push(tx);
        self.trim();
    }

    /// Drop the most recent undo entry without applying it. Useful for
    /// drag-coalesce logic in editor UIs: pop the prior frame's
    /// transaction, then commit a fresh one spanning the whole gesture.
    /// The redo stack is *not* touched (this is not an undo).
    pub fn drop_last_undo(&mut self) -> Option<GraphTransaction<Op>> {
        self.undo.pop()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Discards both stacks. Use after loading a fresh document.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn peek_undo(&self) -> Option<&GraphTransaction<Op>> {
        self.undo.last()
    }

    pub fn peek_redo(&self) -> Option<&GraphTransaction<Op>> {
        self.redo.last()
    }

    /// Labels of every undoable transaction, oldest first.
    pub fn undo_labels(&self) -> impl Iterator<Item = &str> {
        self.undo.iter().map(|t| t.label())
    }

    /// Labels of every redoable transaction; the LAST entry is the next redo.
    pub fn redo_labels(&self) -> impl Iterator<Item = &str> {
        self.redo.iter().map(|t| t.label())
    }

    pub fn peek_undo_label(&self) -> Option<&str> {
        self.undo.last().map(|t| t.label())
    }

    pub fn peek_redo_label(&self) -> Option<&str> {
        self.redo.last().map(|t| t.label())
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub fn history_limit(&self) -> usize {
        self.limit
    }

    fn trim(&mut self) {
        if self.limit > 0 && self.undo.len() > self.limit {
            let overflow = self.undo.len() - self.limit;
            self.undo.drain(..overflow);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Op {
        Set(u32),
        Unset(u32),
    }

    fn tx(label: &str, forward: Vec<Op>, inverse: Vec<Op>) -> GraphTransaction<Op> {
        GraphTransaction::new(label, forward, inverse)
    }

    #[test]
    fn push_rejects_empty_transactions() {
        let mut s = TransactionStack::<Op>::new();
        assert!(!s.push(tx("noop", vec![], vec![])));
        assert!(!s.can_undo());
    }

    #[test]
    fn push_pushes_and_clears_redo() {
        let mut s = TransactionStack::new();
        s.push(tx("first", vec![Op::Set(1)], vec![Op::Unset(1)]));
        s.push(tx("second", vec![Op::Set(2)], vec![Op::Unset(2)]));
        // Simulate one undo so the redo stack is non-empty.
        let popped = s.pop_undo().unwrap();
        s.push_to_redo(popped);
        assert!(s.can_redo());

        // A fresh push clears redo.
        s.push(tx("third", vec![Op::Set(3)], vec![Op::Unset(3)]));
        assert!(!s.can_redo());
    }

    #[test]
    fn history_limit_drops_oldest_entries() {
        let mut s = TransactionStack::with_limit(2);
        for i in 0..5 {
            s.push(tx(&format!("e{i}"), vec![Op::Set(i)], vec![Op::Unset(i)]));
        }
        assert_eq!(s.undo_len(), 2);
        assert_eq!(s.peek_undo_label(), Some("e4"));
    }

    #[test]
    fn a_limit_of_zero_keeps_every_transaction() {
        let mut s = TransactionStack::with_limit(0);
        for i in 0..300 {
            s.push(tx(&format!("e{i}"), vec![Op::Set(i)], vec![Op::Unset(i)]));
        }
        assert_eq!(s.undo_len(), 300);
        assert_eq!(s.peek_undo_label(), Some("e299"));
    }

    #[test]
    fn pop_undo_and_push_to_redo_round_trips() {
        let mut s = TransactionStack::new();
        s.push(tx("e1", vec![Op::Set(1)], vec![Op::Unset(1)]));

        let popped = s.pop_undo().expect("entry present");
        assert_eq!(popped.label(), "e1");
        assert!(!s.can_undo());
        s.push_to_redo(popped);
        assert!(s.can_redo());

        let redone = s.pop_redo().expect("entry present");
        assert_eq!(redone.label(), "e1");
        s.push_to_undo(redone);
        assert!(s.can_undo());
    }

    #[test]
    fn drop_last_undo_returns_entry_without_touching_redo() {
        let mut s = TransactionStack::new();
        s.push(tx("kept", vec![Op::Set(1)], vec![Op::Unset(1)]));
        s.push(tx("dropped", vec![Op::Set(2)], vec![Op::Unset(2)]));
        // Stage a redo entry so we can confirm it survives.
        let popped = s.pop_undo().unwrap();
        s.push_to_redo(popped);
        assert!(s.can_redo());

        let dropped = s.drop_last_undo().expect("entry present");
        assert_eq!(dropped.label(), "kept");
        assert!(s.can_redo(), "drop must not touch redo");
        assert!(!s.can_undo());
    }
}
