use once_cell::sync::Lazy;

pub const CORS_PROXY: &str = "https://proxy.evilrobot.industries/";
pub static COLLECTIONS: Lazy<Vec<(&str, &str, &str, Option<u32>)>> = Lazy::new(|| {
    vec![
        (
            "Azuki",
            "0xed5af388653567af2f388e6224dc7c4b3241c544",
            "https://ikzttp.mypinata.cloud/ipfs/QmQFkLSQysj94s5GvTHPyzTxrawwtjgiiYS2TBLgrvw8CW/",
            Some(10_000),
        ),
        (
            "Beanz",
            "0x306b1ea3ecdf94ab739f1910bbda052ed4a9f949",
            "https://ikzttp.mypinata.cloud/ipfs/QmPZKyuRw4nQTD6S6R5HaNAXwoQVMj8YydDmad3rC985WZ/",
            Some(19_950),
        ),
        (
            "Bored Ape Chemistry Club",
            "0x22c36bfdcef207f9c0cc941936eff94d4246d14a",
            "https://ipfs.io/ipfs/QmdtARLUPQeqXrVcNzQuRqr9UCFoFvn76X9cdTczt4vqfw/",
            None,
        ),
        (
            "Bored Ape Kennel Club",
            "0xba30e5f9bb24caa003e9f2f0497ad287fdf95623",
            "https://ipfs.io/ipfs/QmTDcCdt3yb6mZitzWBmQr65AW6Wska295Dg9nbEYpSUDR/",
            Some(9_602),
        ),
        (
            "Bored Ape Yacht Club",
            "0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d",
            "https://ipfs.io/ipfs/QmeSjSinHpPnmXmspMjwiXyN6zS4E9zccariGR3jxcaWtq/",
            Some(10_000),
        ),
        (
            "Clone X",
            "0x49cf6f5d44e70224e2e23fdcdd2c053f30ada28b",
            "https://clonex-assets.rtfkt.com/",
            Some(19_311),
        ),
        (
            "Cool Cats NFT",
            "0x1a92f7381b9f03921564a437210bb9396471050c",
            "https://api.coolcatsnft.com/cat/",
            Some(9_941),
        ),
        (
            "CrypToadz by GREMPLIN",
            "0x1cb1a5e65610aeff2551a50f76a87a7d3fb649c6",
            "https://arweave.net/OVAmf1xgB6atP0uZg1U0fMd0Lw6DlsVqdvab-WTXZ1Q/",
            Some(7_025),
        ),
        (
            "DeadFellaz",
            "0x2acab3dea77832c09420663b0e1cb386031ba17b",
            "https://api.deadfellaz.io/traits/",
            Some(10_000),
        ),
        (
            "Doodles",
            "0x8a90cab2b38dba80c64b7734e58ee1db38b8992e",
            "https://ipfs.io/ipfs/QmPMc4tcBsMqLRuCQtPmPe84bpSjrC3Ky7t3JWuHXYB4aS/",
            Some(10_000),
        ),
        (
            "Hape Prime",
            "0x4db1f25d3d98600140dfc18deb7515be5bd293af",
            "https://meta.hapeprime.com/",
            Some(8_192),
        ),
        (
            "Meebits",
            "0x7bd29408f11d2bfc23c34f18275bbf23bb716bc7",
            "https://meebits.larvalabs.com/meebit/",
            Some(20_000),
        ),
        (
            "MekaVerse",
            "0x9a534628b4062e123ce7ee2222ec20b86e16ca8f",
            "https://ipfs.io/ipfs/Qmcob1MaPTXUZt5MztHEgsYhrf7R6G7wV8hpcweL8nEfgU/meka/",
            Some(8_888),
        ),
        (
            "Moonbirds",
            "0x23581767a106ae21c074b2276d25e5c3e136a68b",
            "https://live---metadata-5covpqijaa-uc.a.run.app/metadata/",
            Some(10_000),
        ),
        (
            "Mutant Ape Yacht Club",
            "0x60e4d786628fea6478f785a6d7e704777c86a7c6",
            "https://boredapeyachtclub.com/api/mutants/",
            Some(19_423),
        ),
        (
            "ON1 Force",
            "0x3bf2922f4520a8ba0c2efc3d2a1539678dad5e9d",
            "https://ipfs.io/ipfs/QmXgSuLPGuxxRuAana7JdoWmaS25oAcXv3x2pYMN9kVfg3/",
            Some(7_777),
        ),
        (
            "Otherdeed for Otherside",
            "0x34d85c9cdeb23fa97cb08333b511ac86e1c4e258",
            "https://api.otherside.xyz/lands/",
            Some(100_000),
        ),
        (
            "Pudgy Penguins",
            "0xbd3531da5cf5857e7cfaa92426877b022e612cf8",
            "https://ipfs.io/ipfs/QmWXJXRdExse2YHRY21Wvh4pjRxNRQcWVhcKw4DLVnqGqs/",
            Some(8888),
        ),
        (
            "VeeFriends",
            "0xa3aee8bce55beea1951ef834b99f3ac60d1abeeb",
            "https://erc721.veefriends.com/api/metadata/0xa3aee8bce55beea1951ef834b99f3ac60d1abeeb/",
            Some(10_255),
        ),
        (
            "World of Women",
            "0xe785e82358879f061bc3dcac6f0444462d4b5330",
            "https://wow-prod-nftribe.s3.eu-west-2.amazonaws.com/t/",
            Some(10_000),
        ),
    ]
});
