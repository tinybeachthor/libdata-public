mod common;
use common::random_access_memory;

use quickcheck::{quickcheck, Arbitrary, Gen};
use rand::seq::SliceRandom;
use rand::Rng;
use std::u8;

use datacore::{Core, generate_keypair};

const MAX_FILE_SIZE: u32 = 5 * 10;

#[derive(Clone, Debug)]
enum Op {
    Get { index: u32 },
    Append { data: Vec<u8> },
}

impl Arbitrary for Op {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let choices = [0, 1];
        match choices.choose(g).expect("Value should exist") {
            0 => {
                let index: u32 = g.gen_range(0, MAX_FILE_SIZE);
                Op::Get { index }
            }
            1 => {
                let length: u32 = g.gen_range(0, MAX_FILE_SIZE / 3);
                let mut data = Vec::with_capacity(length as usize);
                for _ in 0..length {
                    data.push(u8::arbitrary(g));
                }
                Op::Append { data }
            }
            err => panic!("Invalid choice {}", err),
        }
    }
}

quickcheck! {
    fn implementation_matches_model(ops: Vec<Op>) -> bool {
        async_std::task::block_on(async {
            let keypair = generate_keypair();
            let mut core = Core::new(
                random_access_memory(),
                random_access_memory(),
                random_access_memory(),
                keypair.public, Some(keypair.secret))
                .await.unwrap();
            let mut model = vec![];

            for op in ops {
                match op {
                    Op::Append { data } => {
                        core.append(&data, None)
                            .await.expect("Append should be successful");
                        model.push(data);
                    },
                    Op::Get { index } => {
                        let data = core.get(index)
                            .await.expect("Get should be successful");
                        if index >= core.len() {
                            assert_eq!(data, None);
                        } else {
                            let (data, _) = data.unwrap();
                            assert_eq!(data, model[index as usize].clone());
                        }
                    },
                }
            }
            core.len() as usize == model.len()
        })
    }
}
