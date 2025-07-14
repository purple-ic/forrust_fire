use crate::{
    ashes::{Ashes, BranchId},
    fire::{self, ForestFire},
};

#[test]
fn empty() {
    let fire = ForestFire::<u32>::new();
    let ashes = fire.burn();
    assert!(ashes.root_children().is_empty());
    assert!(ashes.nodes.is_empty())
}

fn make_convoluted() -> ForestFire<u32> {
    let mut fire = ForestFire::<u32>::new();

    /*
    x(0):
        xx(1):
            xxx(6)
        xy(5)
    y(2):
        yx(3):
            yxx(4)
     */

    #[allow(unused)]
    {
        // these are intentionally in a weird order to make it less
        // likely for the tree to accidentally work due to ordering
        let x = fire.branch(fire::BranchId::ROOT, 0);
        let xx = fire.branch(x, 1);
        let y = fire.branch(fire::BranchId::ROOT, 2);
        let yx = fire.branch(y, 3);
        let yxx = fire.branch(yx, 4);
        let xy = fire.branch(x, 5);
        let xxx = fire.branch(xx, 6);
    }

    fire
}

fn assert_convoluted(ashes: &Ashes<u32>) {
    {
        let root = ashes.branch(BranchId::ROOT);
        assert_eq!(root.payload(), None);
        {
            let x = ashes.branch(root.child(0));
            assert_eq!(x.payload(), Some(&0));
            {
                let xx = ashes.branch(x.child(0));
                assert_eq!(xx.payload(), Some(&1));
                {
                    let xxx = ashes.branch(xx.child(0));
                    assert_eq!(xxx.payload(), Some(&6));
                    assert_eq!(xxx.n_children(), 0);
                }
                assert_eq!(xx.n_children(), 1);
            }
            {
                let xy = ashes.branch(x.child(1));
                assert_eq!(xy.payload(), Some(&5));
                assert_eq!(xy.n_children(), 0);
            }
            assert_eq!(x.n_children(), 2);
        }
        {
            let y = ashes.branch(root.child(1));
            assert_eq!(y.payload(), Some(&2));
            {
                let yx = ashes.branch(y.child(0));
                assert_eq!(yx.payload(), Some(&3));
                {
                    let yxx = ashes.branch(yx.child(0));
                    assert_eq!(yxx.payload(), Some(&4));
                    assert_eq!(yxx.n_children(), 0);
                }
                assert_eq!(yx.n_children(), 1);
            }
            assert_eq!(y.n_children(), 1);
        }
    }
}

#[test]
fn convoluted() {
    let fire = make_convoluted();
    let ashes = fire.burn();

    println!("{ashes:#?}\n{}", ashes.print_tree_display());
    assert_convoluted(&ashes);
}

#[cfg(feature = "serde")]
mod serde {
    use serde_json::json;

    use crate::{
        ashes::{Ashes, BranchId},
        test::{assert_convoluted, make_convoluted},
    };

    #[test]
    fn json_de() {
        let json = json!({
            "1": {
                "v": 0,
                "0": {
                    "v": 1
                }
            },
            "0": {
                "1": {
                    "v": 2,
                },
                "v": 3,
                "0": {
                    "v": 3
                },
                "2": {
                    "v": 4
                }
            }
        });
        let ashes = serde_json::from_value::<Ashes<u32>>(json).unwrap();
        println!("deserialized {ashes:#?}");

        let root = ashes.branch(BranchId::ROOT);
        assert_eq!(root.payload(), None);
        {
            let x = ashes.branch(root.child(0));
            assert_eq!(x.payload(), Some(&3));
            {
                let xx = ashes.branch(x.child(0));
                assert_eq!(xx.payload(), Some(&3));
                assert_eq!(xx.n_children(), 0);
            }
            {
                let xy = ashes.branch(x.child(1));
                assert_eq!(xy.payload(), Some(&2));
                assert_eq!(xy.n_children(), 0);
            }
            {
                let xz = ashes.branch(x.child(2));
                assert_eq!(xz.payload(), Some(&4));
                assert_eq!(xz.n_children(), 0);
            }
            assert_eq!(x.n_children(), 3);
        }
        {
            let y = ashes.branch(root.child(1));
            assert_eq!(y.payload(), Some(&0));
            {
                let yx = ashes.branch(y.child(0));
                assert_eq!(yx.payload(), Some(&1));
                assert_eq!(yx.n_children(), 0);
            }
            assert_eq!(y.n_children(), 1);
        }
    }

    #[test]
    fn json_ser_and_de() {
        // ashes to value, value to ashes
        // ashes to ashes, dust to dust (or whatever)
        let fire = make_convoluted();
        let ashes = fire.burn();
        let value = serde_json::to_value(&ashes).unwrap();
        println!("serialized {value:#}");
        assert_eq!(
            value,
            json!({
                "0": {
                    // x
                    "v": 0,
                    "0": {
                        // xx
                        "v": 1,
                        "0": {
                            // xxx
                            "v": 6
                        }
                    },
                    "1": {
                        // xy
                        "v": 5
                    }
                },
                "1": {
                    // y
                    "v": 2,
                    "0": {
                        // yx
                        "v": 3,
                        "0": {
                            // yxx
                            "v": 4,
                        }
                    }
                }
            }),
        );
        let ashes: Ashes<u32> = serde_json::from_value(value).unwrap();
        println!("deserialized {ashes:#?}");
        assert_convoluted(&ashes);
    }
}
