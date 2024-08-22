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
    fn dominates(&self, other_label: &Label) -> bool {
        self.arrival_time <= other_label.arrival_time && self.cost <= other_label.cost
    }
}

#[derive(Clone)]
pub(crate) struct Bag {
    pub(crate) labels: Vec<Label>,
}

impl Bag {
    pub(crate) const fn new() -> Self {
        Bag { labels: Vec::new() }
    }

    // Adds a label to the bag, discarding non-dominated labels. 
    // Returns true if the label was added <=> the bag was modified.
    pub(crate) fn add(&mut self, new_label: Label) -> bool {
        // Remove dominated labels.
        let num_labels = self.labels.len();
        self.labels.retain(|label| !new_label.dominates(label));
        
        // Check if the new label is dominated by any existing label
        if self.labels.iter().any(|label| label.dominates(&new_label)) {
            // If this label is dominated by any existing label, it won't have dominated any existing labels.
            debug_assert!(self.labels.len() == num_labels);
            return false;
        }

        // Add the new label
        self.labels.push(new_label);
        true
    }
    
    pub(crate) fn merge(&mut self, other_bag: &Bag) -> bool {
        let mut updated = false;
        for label in &other_bag.labels {
            updated |= self.add(label.clone());
        }
        updated
    }
}