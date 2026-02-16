use nexwick::model::LeafLabelMap;

#[test]
fn test_get_or_insert_new_label() {
    let mut map = LeafLabelMap::new(5);
    let index_wrybill = map.get_or_insert("Anarhynchus frontalis");
    assert_eq!(index_wrybill, 0);
    assert!(map.contains_label("Anarhynchus frontalis"));
}

#[test]
fn test_get_or_insert_increments_index() {
    let mut map = LeafLabelMap::new(5);
    let index_kaki = map.get_or_insert("Himantopus novaezelandiae");
    let index_pied = map.get_or_insert("Himantopus leucocephalus");
    assert_eq!(index_kaki, 0);
    assert_eq!(index_pied, 1);
    assert_eq!(map.num_labels(), 2);
}

#[test]
fn test_get_or_insert_returns_same_index_for_duplicate() {
    let mut map = LeafLabelMap::new(5);
    let index_kakapo = map.get_or_insert("Strigops habroptilus");
    let index_kea = map.get_or_insert("Nestor notabilis");
    let index_kaka = map.get_or_insert("Nestor meridionalis");
    let index_popoka = map.get_or_insert("Strigops habroptilus");

    assert_eq!(index_kakapo, index_popoka);
    assert_ne!(index_kakapo, index_kea);
    assert_ne!(index_kakapo, index_kaka);
    assert_eq!(map.num_labels(), 3);
}

#[test]
fn test_get_label_returns_correct_label() {
    let mut map = LeafLabelMap::new(5);
    let index_rock_wren = map.get_or_insert("Xenicus gilviventris");
    assert_eq!(map.get_label(index_rock_wren), Some("Xenicus gilviventris"));
}

#[test]
fn test_get_label_returns_none_for_invalid_index() {
    let map = LeafLabelMap::new(5);
    assert_eq!(map.get_label(0), None);
}
