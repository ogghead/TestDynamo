use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;
use spin_sdk::wit::wasi::keyvalue::atomics;
use spin_sdk::wit::wasi::keyvalue::batch;
use spin_sdk::wit::wasi::keyvalue::store::open;

/// A simple Spin HTTP component.
#[http_component]
fn handle_test_dynamo(req: Request) -> anyhow::Result<impl IntoResponse> {
    println!("Handling request to {:?}", req.header("spin-full-url"));

    let store = open("default")?;
    let res = store.exists("foo")?;
    println!("exists? {res}");
    store.set("foo", b"foo")?;
    let res = store.exists("foo")?;
    println!("keys: {:?}", store.list_keys(None)?);

    batch::set_many(
        &store,
        &[
            ("foo".to_string(), b"newnewval".to_vec()),
            ("foo2".to_string(), b"newnewval2".to_vec()),
        ],
    )?;
    let results = batch::get_many(
        &store,
        &["foo".to_string(), "foo2".to_string(), "foo3".to_string()],
    )?;
    assert_eq!(
        results,
        vec![
            ("foo".to_string(), Some(b"newnewval".to_vec())),
            ("foo2".to_string(), Some(b"newnewval2".to_vec())),
        ]
    );
    batch::delete_many(&store, &["foo".to_string(), "foo2".to_string()])?;
    println!("finished all batch ops!");

    println!("keys: {:?}", store.list_keys(None)?);

    let newincr = atomics::increment(&store, "foo2", 1)?;
    println!("incremented {newincr}");

    store.set("foo", b"foo")?;

    let cas = atomics::Cas::new(&store, "foo")?;
    let old_val = cas.current()?;
    assert_eq!(old_val, Some(b"foo".to_vec()));
    atomics::swap(cas, b"foo2")?;

    let cas2 = atomics::Cas::new(&store, "foo")?;
    let old_val = cas2.current()?;
    atomics::swap(cas2, b"foo3")?;

    let new_val = store.get("foo")?;
    assert_eq!(new_val.unwrap().as_slice(), b"foo3");

    let cas3 = atomics::Cas::new(&store, "foonone")?;
    assert!(cas3.current().is_ok());
    let cas4 = atomics::Cas::new(&store, "foonone")?;
    assert!(cas4.current().is_err());
    assert!(atomics::swap(cas3, b"foo5").is_ok());
    println!("swapped foonone");
    assert!(cas4.current().is_ok());

    assert!(atomics::swap(cas4, b"shouldfail").is_ok());

    let new_val = store.get("foonone")?.unwrap();
    assert_eq!(new_val, b"shouldfail");

    store.delete("foonone")?;

    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body("Hello, Fermyon")
        .build())
}
