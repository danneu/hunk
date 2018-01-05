use codec;

lazy_static! {
    static ref CODEC: codec::Codec<'static> = {
        codec::Codec::new(&[
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
            'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't',
            'u', 'v', 'w', 'x', 'y', 'z'
        ])
    };
}

pub fn encode(n: u64) -> String {
    CODEC.encode(n)
}
