// Copyright 2020 IOTA Stiftung
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with
// the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on
// an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and limitations under the License.

use bee_crypto::ternary::Hash;
use bee_tangle::Tangle;
use bee_test::transaction::{create_random_attached_tx, create_random_tx};
use bee_transaction::{bundled::BundledTransaction as Tx, Vertex};

pub struct Transactions {
    pub a: Tx,
    pub b: Tx,
    pub c: Tx,
    pub d: Tx,
    pub e: Tx,
}

pub struct Hashes {
    pub a_hash: Hash,
    pub b_hash: Hash,
    pub c_hash: Hash,
    pub d_hash: Hash,
    pub e_hash: Hash,
}

#[allow(clippy::many_single_char_names)]
pub fn create_test_tangle() -> (Tangle<()>, Transactions, Hashes) {
    // a   b
    // |\ /
    // | c
    // |/|
    // d |
    //  \|
    //   e

    pollster::block_on(async {
        let tangle = Tangle::default();

        let (a_hash, a) = create_random_tx();
        let (b_hash, b) = create_random_tx();
        let (c_hash, c) = create_random_attached_tx(a_hash, b_hash);
        let (d_hash, d) = create_random_attached_tx(a_hash, c_hash);
        let (e_hash, e) = create_random_attached_tx(d_hash, c_hash);

        assert_eq!(*c.trunk(), b_hash);
        assert_eq!(*c.branch(), a_hash);
        assert_eq!(*d.trunk(), c_hash);
        assert_eq!(*d.branch(), a_hash);
        assert_eq!(*e.trunk(), c_hash);
        assert_eq!(*e.branch(), d_hash);

        tangle.insert(a_hash, a.clone(), ()).await;
        tangle.insert(b_hash, b.clone(), ()).await;
        tangle.insert(c_hash, c.clone(), ()).await;
        tangle.insert(d_hash, d.clone(), ()).await;
        tangle.insert(e_hash, e.clone(), ()).await;

        assert_eq!(*tangle.get(&c_hash).await.unwrap().trunk(), b_hash);
        assert_eq!(*tangle.get(&c_hash).await.unwrap().branch(), a_hash);
        assert_eq!(*tangle.get(&d_hash).await.unwrap().trunk(), c_hash);
        assert_eq!(*tangle.get(&d_hash).await.unwrap().branch(), a_hash);
        assert_eq!(*tangle.get(&e_hash).await.unwrap().trunk(), c_hash);
        assert_eq!(*tangle.get(&e_hash).await.unwrap().branch(), d_hash);

        // TODO ensure children reference their parents correctly

        assert_eq!(5, tangle.len());
        assert_eq!(2, tangle.num_children(&a_hash));
        assert_eq!(1, tangle.num_children(&b_hash));
        assert_eq!(2, tangle.num_children(&c_hash));
        assert_eq!(1, tangle.num_children(&d_hash));
        assert_eq!(0, tangle.num_children(&e_hash));

        (
            tangle,
            Transactions { a, b, c, d, e },
            Hashes {
                a_hash,
                b_hash,
                c_hash,
                d_hash,
                e_hash,
            },
        )
    })
}
