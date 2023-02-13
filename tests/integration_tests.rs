extern crate granular_plugin;

const ok: i16 = 200;

#[test]
fn import_works() {
    assert_eq!(ok, granular_plugin::stat());
}
