use once_cell::sync::Lazy;

pub(crate) const CORS_PROXY: &str = "https://proxy.evilrobot.industries/";
pub(crate) static COLLECTIONS: Lazy<Vec<(&str, &str, &str, u32)>> = Lazy::new(|| {
    vec![
        (
            "Azuki",
            "0xed5af388653567af2f388e6224dc7c4b3241c544",
            "https://ikzttp.mypinata.cloud/ipfs/QmQFkLSQysj94s5GvTHPyzTxrawwtjgiiYS2TBLgrvw8CW/",
            10_000,
        ),
        (
            "Bored Ape Yacht Club",
            "0xbc4ca0eda7647a8ab7c2061c2e118a18a936f13d",
            "https://ipfs.io/ipfs/QmeSjSinHpPnmXmspMjwiXyN6zS4E9zccariGR3jxcaWtq/",
            10_000,
        ),
        (
            "Clone X",
            "0x49cf6f5d44e70224e2e23fdcdd2c053f30ada28b",
            "https://clonex-assets.rtfkt.com/",
            19_311,
        ),
        (
            "Cool Cats NFT",
            "0x1a92f7381b9f03921564a437210bb9396471050c",
            "https://api.coolcatsnft.com/cat/",
            9_941,
        ),
        (
            "Doodles",
            "0x8a90cab2b38dba80c64b7734e58ee1db38b8992e",
            "https://ipfs.io/ipfs/QmPMc4tcBsMqLRuCQtPmPe84bpSjrC3Ky7t3JWuHXYB4aS/",
            10_000,
        ),
        (
            "Meebits",
            "0x7bd29408f11d2bfc23c34f18275bbf23bb716bc7",
            "https://meebits.larvalabs.com/meebit/",
            20_000,
        ),
        (
            "Moonbirds",
            "0x23581767a106ae21c074b2276d25e5c3e136a68b",
            "https://live---metadata-5covpqijaa-uc.a.run.app/metadata/",
            10_000,
        ),
        (
            "Mutant Ape Yacht Club",
            "0x60e4d786628fea6478f785a6d7e704777c86a7c6",
            "https://boredapeyachtclub.com/api/mutants/",
            19_423,
        ),
        (
            "Otherdeed for Otherside",
            "0x34d85c9cdeb23fa97cb08333b511ac86e1c4e258",
            "https://api.otherside.xyz/lands/",
            100_000,
        ),
    ]
});
