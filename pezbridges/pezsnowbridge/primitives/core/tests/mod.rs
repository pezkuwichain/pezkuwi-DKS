#[cfg(test)]
mod tests {
	use pezframe_support::traits::Contains;
	use pezsnowbridge_core::AllowSiblingsOnly;
	use xcm::prelude::{Junction::Teyrchain, Location};

	#[test]
	fn allow_siblings_predicate_only_allows_siblings() {
		let sibling = Location::new(1, [Teyrchain(1000)]);
		let child = Location::new(0, [Teyrchain(1000)]);
		assert!(AllowSiblingsOnly::contains(&sibling), "Sibling returns true.");
		assert!(!AllowSiblingsOnly::contains(&child), "Child returns false.");
	}
}
