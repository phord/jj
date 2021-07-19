// Copyright 2021 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::index::IndexRef;
use crate::op_store::RefTarget;
use crate::store::CommitId;

pub fn merge_ref_targets(
    index: IndexRef,
    left: Option<&RefTarget>,
    base: Option<&RefTarget>,
    right: Option<&RefTarget>,
) -> Option<RefTarget> {
    if left == base || left == right {
        right.cloned()
    } else if base == right {
        left.cloned()
    } else {
        let mut adds = vec![];
        let mut removes = vec![];
        if let Some(left) = left {
            adds.extend(left.adds());
            removes.extend(left.removes());
        }
        if let Some(base) = base {
            // Note that these are backwards (because the base is subtracted).
            adds.extend(base.removes());
            removes.extend(base.adds());
        }
        if let Some(right) = right {
            adds.extend(right.adds());
            removes.extend(right.removes());
        }

        while let Some((maybe_remove_index, add_index)) =
            find_pair_to_remove(index, &adds, &removes)
        {
            if let Some(remove_index) = maybe_remove_index {
                removes.remove(remove_index);
            }
            adds.remove(add_index);
        }

        if adds.is_empty() {
            None
        } else if adds.len() == 1 && removes.is_empty() {
            Some(RefTarget::Normal(adds[0].clone()))
        } else {
            Some(RefTarget::Conflict { removes, adds })
        }
    }
}

fn find_pair_to_remove(
    index: IndexRef,
    adds: &[CommitId],
    removes: &[CommitId],
) -> Option<(Option<usize>, usize)> {
    // Removes pairs of matching adds and removes.
    for (add_index, add) in adds.iter().enumerate() {
        for (remove_index, remove) in removes.iter().enumerate() {
            if add == remove {
                return Some((Some(remove_index), add_index));
            }
        }
    }

    // If a "remove" is an ancestor of two different "adds" and one of the
    // "adds" is an ancestor of the other, then pick the descendant.
    for (add_index1, add1) in adds.iter().enumerate() {
        for (add_index2, add2) in adds.iter().enumerate().skip(add_index1 + 1) {
            let first_add_is_ancestor;
            if add1 == add2 || index.is_ancestor(add1, add2) {
                first_add_is_ancestor = true;
            } else if index.is_ancestor(add2, add1) {
                first_add_is_ancestor = false;
            } else {
                continue;
            }
            if removes.is_empty() {
                if first_add_is_ancestor {
                    return Some((None, add_index1));
                } else {
                    return Some((None, add_index2));
                }
            }
            for (remove_index, remove) in removes.iter().enumerate() {
                if first_add_is_ancestor && index.is_ancestor(remove, add1) {
                    return Some((Some(remove_index), add_index1));
                } else if !first_add_is_ancestor && index.is_ancestor(remove, add2) {
                    return Some((Some(remove_index), add_index2));
                }
            }
        }
    }

    None
}