//! Quickly generate a collection of N hashed values whose computed DHT locations
//! are evenly distributed across the space of u32 values.

use crate::*;
use arbitrary::{Arbitrary, Unstructured};
use kitsune_p2p_dht_arc::DhtLocation;
use serde::{Deserialize, Serialize};

/// The total number of items in a collection. This is set to 256 for convenience,
/// but could be changed, or could make this a parameter on a per-collection basis
pub const TOTAL: usize = u8::MAX as usize + 1;
/// The size of each "bucket" represented by each item.
pub const BUCKET_SIZE: usize = ((u32::MAX as u64 + 1) / (TOTAL as u64)) as usize;

/// Container for fixture data generated by this module
#[derive(Debug, Serialize, Deserialize)]
pub struct HashedFixtures<C: HashableContent> {
    /// The generated items
    pub items: Vec<HoloHashed<C>>,
}

impl<T, C> HashedFixtures<C>
where
    C: Clone + HashableContent<HashType = T> + Arbitrary<'static> + std::fmt::Debug + PartialEq,
    T: HashTypeSync,
{
    /// Quickly generate a collection of `N` hashed values whose computed
    /// DHT locations are evenly distributed across the space of u32 values.
    /// Specifically, there is only one hash per interval of size `2^32 / N`
    pub fn generate<F: Fn(&HoloHashed<C>) -> DhtLocation>(
        u: &mut Unstructured<'static>,
        mut fact: Option<contrafact::Facts<'static, C>>,
        relevant_location: F,
    ) -> Self {
        use contrafact::Fact;
        let mut tot = 0;
        let mut items = vec![None; TOTAL];
        while tot < TOTAL {
            let content = if let Some(ref mut fact) = fact {
                fact.build(u)
            } else {
                C::arbitrary(u).unwrap()
            };
            let item = HoloHashed::from_content_sync(content);
            let loc = relevant_location(&item).to_u32();
            let idx = loc as usize / BUCKET_SIZE;

            match &mut items[idx] {
                Some(_) => (),
                h @ None => {
                    *h = Some(item);
                    tot += 1;
                }
            }
        }
        assert!(items.iter().all(|h| h.is_some()));
        let items = items.into_iter().flatten().collect();
        Self { items }
    }

    /// Get the item at the specified "bucket".
    /// There are `self.num` buckets, and the index can be a negative number,
    /// which will be counted backwards from `num`.
    pub fn get(&self, i: i8) -> &HoloHashed<C> {
        &self.items[self.rectify_index(i)]
    }

    /// Get the endpoints for the bucket at the specified index
    pub fn bucket(&self, i: i8) -> (DhtLocation, DhtLocation) {
        let bucket_size = BUCKET_SIZE;
        let start = self.rectify_index(i) * bucket_size;
        (
            DhtLocation::new(start as u32),
            DhtLocation::new((start + bucket_size) as u32),
        )
    }

    fn rectify_index(&self, i: i8) -> usize {
        rectify_index(i)
    }
}

/// Map a signed index into an unsigned index
pub fn rectify_index(i: i8) -> usize {
    if i < 0 {
        (TOTAL as isize + i as isize) as usize
    } else {
        i as usize
    }
}