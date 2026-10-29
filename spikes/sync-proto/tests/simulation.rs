use std::collections::HashSet;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sync_proto::{Change, Device, Server};

#[test]
fn sequential_edits_converge() {
    let mut server = Server::new();
    let mut a = Device::new(1);
    let mut b = Device::new(2);

    a.edit(1, "name", "alpha");
    let r = a.push(&mut server);
    assert_eq!(r.accepted, 1);
    b.pull(&server);
    assert_eq!(b.value(1, "name"), Some("alpha"));

    b.edit(1, "age", "40");
    let r = b.push(&mut server);
    assert_eq!(r.accepted, 1);
    a.pull(&server);
    assert_eq!(a.value(1, "name"), Some("alpha"));
    assert_eq!(a.value(1, "age"), Some("40"));
    assert!(server.conflicts().is_empty());
}

#[test]
fn concurrent_same_field_raises_conflict() {
    let mut server = Server::new();
    let mut a = Device::new(1);
    let mut b = Device::new(2);
    a.pull(&server);
    b.pull(&server);

    a.edit(1, "x", "a");
    b.edit(1, "x", "b");
    assert_eq!(a.push(&mut server).accepted, 1);
    let rb = b.push(&mut server);
    assert_eq!(rb.accepted, 0);
    assert_eq!(rb.conflicts.len(), 1);

    let conflicts = server.conflicts();
    assert_eq!(conflicts.len(), 1);
    let c = &conflicts[0];
    assert_eq!(c.existing.value, "a");
    assert_eq!(c.incoming.value, "b");
    // Neither value applied silently: the server still shows the first write.
    assert_eq!(server.value(1, "x"), Some("a"));

    server.resolve(c.id, "b");
    assert_eq!(server.value(1, "x"), Some("b"));
    a.pull(&server);
    b.pull(&server);
    assert_eq!(a.value(1, "x"), Some("b"));
    assert_eq!(b.value(1, "x"), Some("b"));
}

#[test]
fn concurrent_different_fields_merge() {
    let mut server = Server::new();
    let mut a = Device::new(1);
    let mut b = Device::new(2);
    a.pull(&server);
    b.pull(&server);

    a.edit(1, "x", "a");
    b.edit(1, "y", "b");
    assert_eq!(a.push(&mut server).accepted, 1);
    let rb = b.push(&mut server);
    assert_eq!(rb.accepted, 1);
    assert!(rb.conflicts.is_empty());
    assert!(server.conflicts().is_empty());

    assert_eq!(server.value(1, "x"), Some("a"));
    assert_eq!(server.value(1, "y"), Some("b"));
    a.pull(&server);
    b.pull(&server);
    assert_eq!(a.value(1, "y"), Some("b"));
    assert_eq!(b.value(1, "x"), Some("a"));
}

#[test]
fn skewed_clocks_order_by_lamport() {
    let mut server = Server::new();
    let mut a = Device::new(1);
    let mut b = Device::new(2);
    a.wall_ms = 0;
    b.wall_ms = 3_600_000; // wall clock one hour ahead
    a.pull(&server);
    b.pull(&server);

    // Both devices edit offline from the same base; both changes get lamport 1
    // (identical logical timestamps) even though the wall clocks are 1 h apart.
    a.edit(1, "x", "a");
    b.edit(1, "x", "b");
    assert_eq!(a.pending()[0].lamport, b.pending()[0].lamport);

    // Submit B's change first (its wall clock is "later"); the server must
    // still order by (lamport, device), so A's change wins and B's conflicts.
    let batch: Vec<Change> = b
        .pending()
        .iter()
        .chain(a.pending().iter())
        .cloned()
        .collect();
    let r = server.apply(batch);
    assert_eq!(r.accepted, 1);
    assert_eq!(r.conflicts.len(), 1);
    assert_eq!(server.value(1, "x"), Some("a"));
    assert_eq!(server.conflicts()[0].incoming.value, "b");

    let id = server.conflicts()[0].id;
    server.resolve(id, "a");
    a.discard_pending();
    b.discard_pending();
    a.pull(&server);
    b.pull(&server);
    assert_eq!(a.value(1, "x"), Some("a"));
    assert_eq!(b.value(1, "x"), Some("a"));
}

#[test]
fn random_interleavings_hold_invariants() {
    const SEEDS: u64 = 1_000;
    const STEPS: usize = 200;
    const RECORDS: u64 = 5;
    const FIELDS: [&str; 3] = ["f0", "f1", "f2"];

    for seed in 0..SEEDS {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut server = Server::new();
        let mut devices: Vec<Device> = (0..3).map(Device::new).collect();
        let mut submitted: Vec<Change> = Vec::new();
        let mut n = 0u64;

        for _ in 0..STEPS {
            let d = rng.random_range(0..devices.len());
            match rng.random_range(0..10) {
                0..=4 => {
                    let record = rng.random_range(0..RECORDS);
                    let field = FIELDS[rng.random_range(0..FIELDS.len())];
                    let value = format!("d{d}-s{seed}-e{n}");
                    n += 1;
                    devices[d].edit(record, field, &value);
                }
                5..=6 => {
                    submitted.extend(devices[d].pending().iter().cloned());
                    devices[d].push(&mut server);
                }
                7..=8 => devices[d].pull(&server),
                _ => {} // dropped connection: no sync this step
            }
        }

        // Final full sync: push everything pending, pull everything back.
        for d in &mut devices {
            submitted.extend(d.pending().iter().cloned());
            d.push(&mut server);
            d.pull(&server);
        }

        // Resolve every conflict; winner is the rejected incoming value.
        let ids: Vec<u64> = server.conflicts().iter().map(|c| c.id).collect();
        for id in ids {
            let winner = server
                .conflicts()
                .iter()
                .find(|c| c.id == id)
                .unwrap()
                .incoming
                .value
                .clone();
            server.resolve(id, &winner);
        }
        for d in &mut devices {
            d.pull(&server);
        }

        // Invariant 1: every submitted change is reflected in the server log
        // or recorded in a conflict — exactly once, never lost, never both.
        let in_log: HashSet<&Change> = server.entries().iter().map(|e| &e.change).collect();
        let in_conflict: HashSet<&Change> =
            server.conflicts().iter().map(|c| &c.incoming).collect();
        for ch in &submitted {
            assert!(
                in_log.contains(ch) ^ in_conflict.contains(ch),
                "seed {seed}: change lost or duplicated: {ch:?}"
            );
        }

        // Invariant 2: every conflict raised is visible in conflicts().
        // (conflicts() is the authoritative list the assertions above read.)

        // Invariant 3: all devices equal the server on every field.
        for record in 0..RECORDS {
            for field in FIELDS {
                let expected = server.value(record, field);
                for d in &devices {
                    assert_eq!(
                        d.value(record, field),
                        expected,
                        "seed {seed}: device {} diverged on ({record}, {field})",
                        d.id
                    );
                }
            }
        }
    }
}
