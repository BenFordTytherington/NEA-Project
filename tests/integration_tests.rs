extern crate granular_plugin;

const OK: i16 = 200;

#[test]
fn import_works() {
    assert_eq!(OK, granular_plugin::stat());
}
