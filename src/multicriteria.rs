use std::iter::repeat;
use arrayvec::ArrayVec;
use crate::journey::Boarding;
use crate::network::{PathfindingCost, Timestamp};

// Overengineered version:
/*
pub(crate) trait BagTrait<L> {
    //fn add(&mut self, label: &impl TwoLabelTrait);
    fn iter<'a>(&'a self) -> impl Iterator<Item=&'a L>
    where
        L: 'a;
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item=&'a mut L>
    where
        L: 'a;
}

pub(crate) trait LabelTrait: Clone {
    fn dominated_by(&self, other: &Self) -> bool;
}

pub(crate) trait TwoLabelTrait: LabelTrait {
    fn time(&self) -> Timestamp;
    fn cost(&self) -> PathfindingCost;
}

// #[derive(Clone)]
// pub(crate) struct StopLabel {
//     time: Timestamp,
//     cost: PathfindingCost,
// }
//
// impl StopLabel {
//     pub fn new(time: Timestamp, cost: PathfindingCost) -> Self {
//         StopLabel { time, cost }
//     }
// }
// impl LabelTrait for StopLabel {
//     fn dominated_by(&self, other: &Self) -> bool {
//         self.time >= other.time && self.cost >= other.cost
//     }
// }

#[derive(Clone)]
pub(crate) struct TwoLabel<M: Clone> {
    pub time: Timestamp,
    pub cost: PathfindingCost,
    pub metadata: M,
}

impl<M: Clone> TwoLabel<M> {
    pub fn new(time: Timestamp, cost: PathfindingCost, metadata: M) -> Self {
        TwoLabel { time, cost, metadata }
    }
}

impl<M: Clone> TwoLabelTrait for TwoLabel<M> {
    fn time(&self) -> Timestamp { self.time }

    fn cost(&self) -> PathfindingCost { self.cost }
}

// Allow automatic conversion from a label with metadata to a label without metadata.
impl<M> From<TwoLabel<M>> for TwoLabel<()> {
    fn from(label: TwoLabel<M>) -> Self {
        TwoLabel {
            time: label.time,
            cost: label.cost,
            metadata: (),
        }
    }
}

impl<M: Clone> LabelTrait for TwoLabel<M> {
    fn dominated_by(&self, other: &Self) -> bool {
        self.time >= other.time && self.cost >= other.cost
    }
}

//pub(crate) type StopLabel = TwoLabel<()>;
pub(crate) type Label = TwoLabel<Option<TripIndex>>;

// A set of two non-dominating labels.
pub(crate) struct TwoLabelBag<L: TwoLabelTrait> {
    time_optimised_label: Option<L>,
    cost_optimised_label: Option<L>,
}

impl<L: TwoLabelTrait> TwoLabelBag<L> {
    pub fn new() -> Self {
        TwoLabelBag {
            time_optimised_label: None,
            cost_optimised_label: None,
        }
    }
}

//impl<'a, L: LabelTrait> Iterator for BagIterator<'a, TwoLabelBag<L>> {
//    type Item = &'a mut L;
//
//    fn next(&mut self) -> Option<Self::Item> {
//        match self.state {
//            TwoLabelBagIteratorState::Initial => {
//                if let Some(label) = &mut self.bag.time_optimised_label {
//                    self.state = TwoLabelBagIteratorState::TimeOptimised;
//                    Some(label)
//                } else if let Some(label) = &mut self.bag.cost_optimised_label {
//                    self.state = TwoLabelBagIteratorState::CostOptimised;
//                    Some(label)
//                } else {
//                    None
//                }
//            },
//            TwoLabelBagIteratorState::TimeOptimised => {
//                if let Some(label) = &mut self.bag.cost_optimised_label {
//                    self.state = TwoLabelBagIteratorState::CostOptimised;
//                    Some(label)
//                } else {
//                    None
//                }
//            },
//            TwoLabelBagIteratorState::CostOptimised => {
//                None
//            },
//        }
//    }
//}

//impl std::ops::AddAssign<TwoLabelBag<RouteLabel>> for TwoLabelBag<StopLabel> {
//    fn add_assign(&mut self, other: TwoLabelBag<RouteLabel>) {
//        if let Some(label) = &other.time_optimised_label {
//            self.add(label);
//        }
//        if let Some(label) = &other.cost_optimised_label {
//            self.add(label);
//        }
//    }
//}

impl<L: TwoLabelTrait> BagTrait<L> for TwoLabelBag<L> {
    //fn add(&mut self, label: &TwoLabelTrait) {
        // Check if dominated by any of the labels.
        // if let Some(label0) = &self.time_optimised_label {
        //     if label.dominated_by(label0) {
        //         return;
        //     }
        // } else if let Some(label1) = &self.cost_optimised_label {
        //     if label.dominated_by(label1) {
        //         return;
        //     }
        // }
        // if self.time_optimised_label.is_none() {
        //     self.time_optimised_label = Some(label);
        // } else if self.cost_optimised_label.is_none() {
        //     if label.dominated_by(self.time_optimised_label.as_ref().unwrap()) {
        //         return;
        //     }
        //     self.cost_optimised_label = Some(label);
        // } else {
        //     let label0 = self.time_optimised_label.as_ref().unwrap();
        //     let label1 = self.cost_optimised_label.as_ref().unwrap();
        //     if label.time < label0.time {
        //         self.cost_optimised_label = Some(label0.clone());
        //         self.time_optimised_label = Some(label);
        //     } else if label.time < label1.time {
        //         self.cost_optimised_label = Some(label);
        //     }
        // }
    //}

    fn iter<'a>(&'a self) -> impl Iterator<Item=&'a L>
    where
        L: 'a,
    {
        once(&self.time_optimised_label).chain(once(&self.cost_optimised_label)).flatten()
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item=&'a mut L>
    where
        L: 'a,
    {
        once(&mut self.time_optimised_label).chain(once(&mut self.cost_optimised_label)).flatten()
    }
}

//pub(crate) type StopBag = TwoLabelBag<StopLabel>;
pub(crate) type Bag = TwoLabelBag<Label>; // RouteBag

impl Bag {
    // Adds a label to the bag, discarding non-dominated labels.
    pub(crate) fn add(&self, label: &Label) {

    }
}

*/

#[derive(Clone)]
pub(crate) struct Label {
    pub(crate) arrival_time: Timestamp,
    pub(crate) cost: PathfindingCost,
    pub(crate) boarding: Option<Boarding>,
}

impl Label {
    pub fn new(arrival_time: Timestamp, cost: PathfindingCost) -> Self {
        Label { arrival_time, cost, boarding: None }
    }
    fn dominates(&self, other_label: &Label) -> bool {
        self.arrival_time <= other_label.arrival_time && self.cost <= other_label.cost
    }
}

#[derive(Clone)]
pub(crate) struct Bag<const N: usize = 5> {
    // Labels are sorted by increasing arrival time. Only non-dominated labels are stored, so labels end up also sorted in decreasing cost.
    // Labels are stored in a fixed-size array to avoid heap allocation. Worst arrival time labels are discarded.
    pub(crate) labels: ArrayVec<Label, N>,
}

impl Bag {
    pub(crate) const fn new() -> Self {
        Bag { labels: ArrayVec::new_const() }
    }

    pub(crate) fn dominates(&self, other_label: &Label) -> bool {
        for label in &self.labels {
            if label.dominates(other_label) {
                return true;
            }
        }
        false
    }

    // OLD ADD FUNCTION
    // Adds a label to the bag, discarding non-dominated labels.
    // Returns true if the label was added <=> the bag was modified.
    //pub(crate) fn add2(&mut self, new_label: Label) -> bool {
    //    if self.labels.is_empty() {
    //        self.labels.push(new_label);
    //        return true;
    //    }

    //    // Remove dominated labels.
    //    let num_labels = self.labels.len();
    //    self.labels.retain(|label| !new_label.dominates(label));

    //    // Check if the new label is dominated by any existing label
    //    if self.labels.iter().any(|label| label.dominates(&new_label)) {
    //        // If this label is dominated by any existing label, it won't have dominated any existing labels.
    //        debug_assert!(self.labels.len() == num_labels);
    //        return false;
    //    }

    //    // Hack for ArrayVec:
    //    if self.labels.is_full() {
    //        false
    //    } else {
    //        // Add the new label
    //        self.labels.push(new_label);
    //        true
    //    }
    //}

    // pub(crate) fn merge(&mut self, other_bag: &Bag) -> bool {
    //     let mut updated = false;
    //     for label in &other_bag.labels {
    //         updated |= self.add(label.clone());
    //     }
    //     updated
    // }
}

impl<const N: usize> Bag<N> {
    // Adds a label to the bag, discarding non-dominated labels.
    // Returns true if the label was added <=> the bag was modified.
    pub(crate) fn add(&mut self, new_label: Label) -> bool {
        if self.labels.is_empty() {
            self.labels.push(new_label);
            return true;
        }
        // At least one label is present.

        // Position of the first label with a larger arrival time than the new label.
        let partition = self.labels.iter().position(|label| new_label.arrival_time < label.arrival_time);
        let is_last_label = partition.is_none();
        let partition = partition.unwrap_or(self.labels.len());

        // All the labels before the partition have a smaller arrival time than the new label, and may dominate it.
        if self.labels[..partition].iter().any(|label| label.cost <= new_label.cost) {
            // The new label is dominated by at least one existing label.
            false
        } else {
            // The new label is not dominated. Remove existing labels that are dominated by the new label.

            if !is_last_label {
                // All the labels after the partition have a larger arrival time than the new label, so only keep ones with a smaller cost.
                let keep = self.labels.iter().skip(partition).map(|label| label.cost < new_label.cost).collect::<ArrayVec<_, N>>();
                let mut keep_iter = repeat(true).take(partition).chain(keep.into_iter());
                debug_assert!(keep_iter.size_hint().0 == self.labels.len());
                self.labels.retain(|_| keep_iter.next().unwrap());
            }

            // Arrival times are unique, so if a label exists with the same arrival time as the new label, it must be the label before the partition.
            if partition > 0 {
                let previous_label = &mut self.labels[partition - 1];
                if previous_label.arrival_time == new_label.arrival_time {
                    // The new label has the same arrival time as the previous label.
                    // If the new label has a smaller cost, replace the previous label.
                    if new_label.cost < previous_label.cost {
                        *previous_label = new_label;
                        return true;
                    } else {
                        // The new label is dominated by the previous label.
                        unreachable!("The new label should have been dominated in the previous check.");
                    };
                }
            }

            // Add the new label.
            if self.labels.is_full() {
                if is_last_label {
                    if new_label.arrival_time < self.labels.last().unwrap().arrival_time {
                        // Prioritise arrival time over cost. Add the new label if it has a smaller arrival time than the last label.
                        self.labels.pop();
                    } else {
                        // Don't add the last label.
                        return false;
                    }
                } else {
                    // Pop off last label to make space for the new label.
                    self.labels.pop();
                }
            }

            self.labels.insert(partition, new_label);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bag_add() {
        let mut bag = Bag::new();

        // Should always add the first label.
        assert_eq!(bag.add(Label::new(5, 5.)), true);   // 1
        assert_eq!(bag.labels.len(), 1);

        // Should not add existing labels.
        assert_eq!(bag.add(Label::new(5, 5.)), false);  // 2
        assert_eq!(bag.labels.len(), 1);

        // Should not add dominated labels.
        assert_eq!(bag.add(Label::new(12, 9.)), false); // 3
        assert_eq!(bag.add(Label::new(9, 12.)), false); // 4
        assert_eq!(bag.add(Label::new(5, 7.)), false);  // 5
        assert_eq!(bag.add(Label::new(7, 5.)), false);  // 6
        assert_eq!(bag.labels.len(), 1);

        // Should add non-dominated labels.
        assert_eq!(bag.add(Label::new(7, 3.)), true);   // 7
        assert_eq!(bag.add(Label::new(4, 10.)), true);  // 8
        assert_eq!(bag.add(Label::new(3, 50.)), true);  // 9
        assert_eq!(bag.labels.len(), 4);

        // Should dominate existing labels.
        assert_eq!(bag.add(Label::new(2, 5.)), true);   // 10 dominates 1, 8, 9.
        assert_eq!(bag.add(Label::new(1, 4.5)), true);  // 11 dominates 10.
        assert_eq!(bag.labels.len(), 2);

        // Should replace existing labels with the same arrival time if the new label has a lower cost.
        assert_eq!(bag.add(Label::new(7, 2.5)), true);  // 12
        assert_eq!(bag.add(Label::new(7, 2.4)), true);  // 13
        assert_eq!(bag.add(Label::new(7, 2.6)), false); // 14
        assert_eq!(bag.labels.len(), 2);

        // Should discard the last label if the bag is full and the new label has a smaller arrival time.
        assert_eq!(bag.add(Label::new(8, 1.9)), true);   // 15
        assert_eq!(bag.add(Label::new(9, 1.8)), true);   // 16
        assert_eq!(bag.add(Label::new(10, 1.7)), true);  // 17
        assert_eq!(bag.labels.len(), 5);
        assert_eq!(bag.add(Label::new(6, 4.)), true);    // 18 discards 17.
        assert_eq!(bag.labels.len(), 5);
    }
}