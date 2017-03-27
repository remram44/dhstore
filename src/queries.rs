//! # Queries
//!
//! This module contains the query logic: the query structures, running queries
//! against objects, and parsing text queries.
//!
//! ## Syntax
//!
//! Because the object store is essentially a graph, a query is a sequence of
//! selectors that operate on the results of the previous one, starting from one
//! of the roots that are known to the system.
//!
//! Example of queries:
//!
//! Find contacts aged over 25, whose phone number begin with with 347:
//!
//! ```text
//! @|.contacts|.age>25|(.phones.number~"347")
//! ```
//!
//! Find the backed up files whose name match a 20-year-old contact's firstname:
//!
//! ```text
//! @|.contacts|.age=20|.firstname@|.backups
//! ```
//!
//! Find recently deleted pictures of cats:
//!
//! ```text
//! @log|.[@photos]|.date>now-1week|.tags<["cat"]
//! ```
//!
//! Anything linking to a specific picture:
//!
//! ```text
//! @all|values|.=(@photos|id=DOdY4OwCEf6AouK4eK6fRs)
//! ```

use std::rc::{Rc, Weak};

use common::{ID, Property};

/// A result from advancing the query.
///
/// Might be an object, another property value, or an error (type error when
/// indexing, failed assertion, missing key or index, ...). It might also be
/// nothing at all, in which case we should try to advance again.
pub type QueryResult = Result<Option<Property>, &'static str>;

type MaybeQueryResult = Result<Option<Property>, Option<&'static str>>;

/// A full query, that can be a combination of many subqueries and filters.
pub struct Query {
    roots: Vec<FilterNode>,
    pos: usize,
}

impl Query {
    /// Advance the query, computing stuff and possibly returning a result.
    pub fn advance(&mut self) -> QueryResult {
        loop {
            match self.roots[self.pos].advance() {
                // Error
                Err(Some(e)) => return Err(e),
                // Value
                Ok(Some(p)) => {
                    assert_eq!(self.pos, self.roots.len() - 1,
                            "Filter chain returned a value early");
                    return Ok(Some(p))
                }
                // Nothing yet
                Err(None) => {
                    if self.pos + 1 < self.roots.len() {
                        self.pos += 1;
                    }
                }
                // This root is done
                Ok(None) => {
                    if self.pos == 0 {
                        return Ok(None);
                    }
                    self.roots[self.pos].reset();
                    self.pos -= 1;
                }
            }
        }
    }
}

/// A root filter of a query, `@something`.
///
/// This is always the start point of a chain of filters, where the data comes
/// from; it doesn't take any inputs.
struct StoreRoot {
    name: String,
}

trait Filter {
    fn advance(&mut self, values: Vec<&Property>) -> MaybeQueryResult;
    fn reset(&mut self) {}
}

struct FilterNode {
    filter: Box<Filter>,
    value: Option<Property>,
    prev_filters: Vec<Weak<FilterNode>>,
    next_filters: Vec<(Rc<FilterNode>, usize)>,
}

impl FilterNode {
    fn advance(&mut self) -> MaybeQueryResult {
        let refs = self.prev_filters.iter()
            .map(|w| w.upgrade().unwrap())
            .collect::<Vec<_>>();
        let values = refs.iter()
            .map(|f| f.value.as_ref().unwrap())
            .collect::<Vec<&Property>>();
        self.filter.advance(values)
    }

    fn reset(&mut self) {
        self.filter.reset();
    }
}

/*struct Index {}

impl Filter for Index {
    fn advance(&mut self, values: Vec<&Property>) -> MaybeQueryResult {
        assert!(values.len() == 2);

        match (values[0], values[1]) {
            (Property::Dict(d), Property::String(s)) => {
                debug!("indexing dict with {:?}", s);
                d.get(s).ok_or("missing key")
            }
            (Property::List(l), Property::Integer(i)) => {
                if i < 0 {
                    Err("negative index")
                } else if i >= l.len() {
                    Err("index out of bound")
                } else {
                    debug!("indexing list with {:?}", i);
                    l[i]
                }
            }
            (Property::Dict(_), _) => Err("invalid key type"),
            (Property::List(_), _) => Err("invalid index type"),
            (_, _) => Err("invalid container type"),
        }
    }
}*/
