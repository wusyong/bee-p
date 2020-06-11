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

#[cfg(test)]
mod tests {
    use bee_crypto::{Kerl, Sponge};
    use bee_ternary::{T1B1Buf, T3B1Buf, TritBuf, TryteBuf};

    macro_rules! test_kerl {
        ($test_name:ident, $input_trytes:expr, $output_trytes:expr) => {
            #[test]
            fn $test_name() {
                let input = $input_trytes;
                let output = $output_trytes;

                let mut kerl = Kerl::new();

                let input_trytes = TryteBuf::try_from_str(input);
                assert!(input_trytes.is_ok());
                let input_trytes = input_trytes.unwrap();

                let input_trit_buf = input_trytes.as_trits().encode::<T1B1Buf>();

                let expected_hash = TryteBuf::try_from_str(output);
                assert!(expected_hash.is_ok());
                let expected_hash = expected_hash.unwrap();

                assert!(kerl.absorb(input_trit_buf.as_slice()).is_ok());

                let output_len = expected_hash
                    .as_trits()
                    .len();
                let mut calculated_hash = TritBuf::<T1B1Buf>::zeros(output_len);
                assert!(kerl.squeeze_into(&mut calculated_hash.as_slice_mut()).is_ok());

                let calculated_hash = calculated_hash.encode::<T3B1Buf>();

                assert_eq!(calculated_hash.as_slice(), expected_hash.as_trits());
            }
        };

        ( $( $test_name:ident: $input_trytes:expr => $output_trytes:expr ),+ $(,)?) => {
            $(
                test_kerl!($test_name, $input_trytes, $output_trytes);
            )+
        }
    }

    test_kerl!(
        from_iota_go_normal_trytes_1:
        "HHPELNTNJIOKLYDUW9NDULWPHCWFRPTDIUWLYUHQWWJVPAKKGKOAZFJPQJBLNDPALCVXGJLRBFSHATF9C"
        =>
        "DMJWZTDJTASXZTHZFXFZXWMNFHRTKWFUPCQJXEBJCLRZOM9LPVJSTCLFLTQTDGMLVUHOVJHBBUYFD9AXX",
        from_iota_go_normal_trytes_2:
        "QAUGQZQKRAW9GKEFIBUD9BMJQOABXBTFELCT9GVSZCPTZOSFBSHPQRWJLLWURPXKNAOWCSVWUBNDSWMPW"
        =>
        "HOVOHFEPCIGTOFEAZVXAHQRFFRTPQEEKANKFKIHUKSGRICVADWDMBINDYKRCCIWBEOPXXIKMLNSOHEAQZ",
        from_iota_go_normal_trytes_3:
        "MWBLYBSRKEKLDHUSRDSDYZRNV9DDCPN9KENGXIYTLDWPJPKBHQBOALSDH9LEJVACJAKJYPCFTJEROARRW"
        =>
        "KXBKXQUZBYZFSYSPDPCNILVUSXOEHQWWWFKZPFCQ9ABGIIQBNLSWLPIMV9LYNQDDYUS9L9GNUIYKYAGVZ",
        from_iota_go_output_with_non_zero_243rd_trit:
        "GYOMKVTSNHVJNCNFBBAH9AAMXLPLLLROQY99QN9DLSJUHDPBLCFFAIQXZA9BKMBJCYSFHFPXAHDWZFEIZ"
        =>
        "OXJCNFHUNAHWDLKKPELTBFUCVW9KLXKOGWERKTJXQMXTKFKNWNNXYD9DMJJABSEIONOSJTTEVKVDQEWTW",
        from_iota_go_input_with_243_trits:
        "EMIDYNHBWMBCXVDEFOFWINXTERALUKYYPPHKP9JJFGJEIUY9MUDVNFZHMMWZUYUSWAIOWEVTHNWMHANBH"
        =>
        "EJEAOOZYSAWFPZQESYDHZCGYNSTWXUMVJOVDWUNZJXDGWCLUFGIMZRMGCAZGKNPLBRLGUNYWKLJTYEAQX",
        from_iota_go_output_with_more_than_243_trits:
        "9MIDYNHBWMBCXVDEFOFWINXTERALUKYYPPHKP9JJFGJEIUY9MUDVNFZHMMWZUYUSWAIOWEVTHNWMHANBH"
        =>
        "G9JYBOMPUXHYHKSNRNMMSSZCSHOFYOYNZRSZMAAYWDYEIMVVOGKPJBVBM9TDPULSFUNMTVXRKFIDOHUXXVYDLFSZYZTWQYTE9SPYYWYTXJYQ9IFGYOLZXWZBKWZN9QOOTBQMWMUBLEWUEEASRHRTNIQWJQNDWRYLCA",
        from_iota_go_input_and_output_with_more_than_243_trits:
        "G9JYBOMPUXHYHKSNRNMMSSZCSHOFYOYNZRSZMAAYWDYEIMVVOGKPJBVBM9TDPULSFUNMTVXRKFIDOHUXXVYDLFSZYZTWQYTE9SPYYWYTXJYQ9IFGYOLZXWZBKWZN9QOOTBQMWMUBLEWUEEASRHRTNIQWJQNDWRYLCA"
        =>
        "LUCKQVACOGBFYSPPVSSOXJEKNSQQRQKPZC9NXFSMQNRQCGGUL9OHVVKBDSKEQEBKXRNUJSRXYVHJTXBPDWQGNSCDCBAIRHAQCOWZEBSNHIJIGPZQITIBJQ9LNTDIBTCQ9EUWKHFLGFUVGGUWJONK9GBCDUIMAYMMQX",
        negative_byte_input:
        "DJ9WGAKRZOMH9KVRCHGCDCREXZVDKY9FXAXVSLELYADXHQCQQSMQYAEEBTEIWTQDUZIOFSFLBQQA9RUPX"
        =>
        "XRZCRWFXU9UYRKFQRKWROIRGEVGTUGUBKDYGPWDTUXXOFVXWRTQBRRGGUSIEMPAISTUEYEZJXXEPUTY9D",
    );
}
