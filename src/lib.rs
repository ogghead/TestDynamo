use spin_sdk::http::{IntoResponse, Request, Response};
use spin_sdk::http_component;
use spin_sdk::wit::wasi::keyvalue::{atomics, batch, store::open};

/// A simple Spin HTTP component.
#[http_component]
fn handle_test_dynamo(req: Request) -> anyhow::Result<impl IntoResponse> {
    println!("Handling request to {:?}", req.header("spin-full-url"));

    let store = open("default")?;

    let key = "foo";
    let key2 = "foo2";
    let val = b"valfoo";
    let val2 = b"valfoo2";

    println!("Simple ops");

    println!("Delete");
    store.delete(key)?;

    println!("Checking existence of deleted object");
    assert!(!(store.exists(key)?));

    println!("Get on deleted object");
    assert_eq!(store.get(key)?, None);

    println!("Put");
    store.set(key, val)?;

    println!("Exists on existing object");
    assert!((store.exists(key)?));

    println!("Get on existing object");
    assert_eq!(store.get(key)?.unwrap(), val);

    println!("Cleanup");
    store.delete(key)?;

    println!("Bulk ops");

    println!("Bulk delete");
    batch::delete_many(&store, &[key.to_owned(), key2.to_owned()])?;

    println!("Bulk get on missing objects");
    assert!(batch::get_many(&store, &[key.to_owned(), key2.to_owned()])?.is_empty());

    println!("Bulk set");
    batch::set_many(
        &store,
        &[
            (key.to_owned(), val.to_vec()),
            (key2.to_owned(), val2.to_vec()),
        ],
    )?;

    println!("Bulk get on existing objects");
    assert_eq!(
        batch::get_many(&store, &[key.to_owned(), key2.to_owned()])?,
        vec![
            (key.to_owned(), Some(val.to_vec())),
            (key2.to_owned(), Some(val2.to_vec())),
        ]
    );

    println!("Cleanup");
    batch::delete_many(&store, &[key.to_owned(), key2.to_owned()])?;

    println!("Atomics");
    store.delete(key)?;

    println!("Increment");
    assert_eq!(atomics::increment(&store, key, 1)?, 1);
    assert_eq!(atomics::increment(&store, key, 1)?, 2);

    store.delete(key)?;

    println!(
        "Two handles, missing and unknown object (no read), both write successfully but atomically"
    );
    let cas1 = atomics::Cas::new(&store, key)?;
    let cas2 = atomics::Cas::new(&store, key)?;

    println!("Succeeds 1");
    atomics::swap(cas1, val)?;
    println!("Succeeds 2");
    atomics::swap(cas2, val2)?;

    store.delete(key)?;

    store.set(key, val)?;

    println!(
        "Two handles, existing but unknown object (no read), both write successfully but atomically"
    );
    let cas1 = atomics::Cas::new(&store, key)?;
    let cas2 = atomics::Cas::new(&store, key)?;

    println!("Succeeds 1");
    atomics::swap(cas1, val)?;
    println!("Succeeds 2");
    atomics::swap(cas2, val2)?;

    println!("Two handles, read object with version, only one writes successfully");
    let cas1 = atomics::Cas::new(&store, key)?;
    let cas2 = atomics::Cas::new(&store, key)?;
    cas1.current()?;
    cas2.current()?;

    println!("Succeeds 1");
    atomics::swap(cas1, val)?;

    println!("Fails 2");
    assert!(atomics::swap(cas2, val2).is_err());

    println!("Two handles, read object without version, only one writes successfully");

    println!("Set to unversioned");
    store.set(key, val)?;

    let cas1 = atomics::Cas::new(&store, key)?;
    let cas2 = atomics::Cas::new(&store, key)?;
    cas1.current()?;
    cas2.current()?;

    println!("Succeeds 1");
    atomics::swap(cas1, val2)?;

    println!("Fails 2");
    assert!(atomics::swap(cas2, val2).is_err());

    println!("Two handles, read nonexistent object, only one writes successfully");
    store.delete(key)?;

    let cas1 = atomics::Cas::new(&store, key)?;
    let cas2 = atomics::Cas::new(&store, key)?;
    cas1.current()?;
    cas2.current()?;

    println!("Succeeds 1");
    atomics::swap(cas1, val)?;

    println!("Fails 2");
    let res = atomics::swap(cas2, val2);
    assert!(res.is_err());
    if let Err(atomics::CasError::CasFailed(newcas2)) = res {
        atomics::swap(newcas2, val2)?;
    }

    println!("Cleanup");
    store.delete(key)?;

    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body("Hello, Fermyon")
        .build())
}
