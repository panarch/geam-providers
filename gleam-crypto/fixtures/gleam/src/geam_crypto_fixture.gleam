import gleam/bit_array
import gleam/crypto

pub fn main() {
  assert verify()
}

pub fn verify() -> Bool {
  vector(
    crypto.Sha224,
    "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7",
    "88ff8b54675d39b8f72322e65ff945c52d96379988ada25639747e69",
  )
  vector(
    crypto.Sha256,
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8",
  )
  vector(
    crypto.Sha384,
    "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
    "d7f4727e2c0b39ae0f1e40cc96f60242d5b7801841cea6fc592c5d3e1ae50700582a96cf35e1e554995fe4e03381c237",
  )
  vector(
    crypto.Sha512,
    "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
    "b42af09057bac1e2d41708e48a902e09b5ff7f12ab428a4fe86653c73dd248fb82f948a549f7b791a5b41915ee4d1ec3935357e4e2317250d0372afa2ebeeb3a",
  )
  vector(
    crypto.Sha1,
    "a9993e364706816aba3e25717850c26c9cd0d89d",
    "de7c9b85b8b78aa6bc8a7a36f70a90701c9db4d9",
  )
  vector(
    crypto.Md5,
    "900150983cd24fb0d6963f7d28e17f72",
    "80070713463e7749b90c2dc24911e275",
  )

  signed_vector(
    crypto.Sha224,
    "SFMyMjQ.aGVsbG8.eTJguGoJkR4mTKNHe_cx7-jXqqQIO4r_P6FPqg",
  )
  signed_vector(
    crypto.Sha256,
    "SFMyNTY.aGVsbG8.7h6Axw4pHo2rZrjleNFip-z_qKiQ-WP6UK80AJgvaFs",
  )
  signed_vector(
    crypto.Sha384,
    "SFMzODQ.aGVsbG8.FXgIOXQc1Qs4ycSGO94-eKuRQ_rXX3-bhmpU6_FrKbUkt6YEpiKZ6kFKtHz4Zn_O",
  )
  signed_vector(
    crypto.Sha512,
    "SFM1MTI.aGVsbG8.VDllPKdSedHzOirjB62FRPlEkwsX81Vv91iI-D23u1uhk5a6ktxtpU7pi6Z2p-BM5W0LMjaKsaakwZFBI5nDjw",
  )
  signed_vector(crypto.Sha1, "SFMx.aGVsbG8.M0AGOKK4uV8SlaV6Hl4jZSDhNgk")
  signed_vector(crypto.Md5, "SE1ENQ.aGVsbG8.8Q7NKhl2YbXDPQ0WIoVIKw")

  let root = crypto.new_hasher(crypto.Sha256)
  let same_root = root
  let left = crypto.hash_chunk(root, <<"a":utf8>>)
  let right = crypto.hash_chunk(root, <<"b":utf8>>)
  let complete = crypto.hash_chunk(left, <<"bc":utf8>>)
  assert same_root == root
  assert root != left
  assert left != right
  assert crypto.digest(complete) == crypto.hash(crypto.Sha256, <<"abc":utf8>>)
  assert crypto.digest(complete) == crypto.digest(complete)
  assert crypto.digest(left) != crypto.digest(right)

  let signed =
    crypto.sign_message(<<"hello":utf8>>, <<"secret":utf8>>, crypto.Sha256)
  assert signed == "SFMyNTY.aGVsbG8.7h6Axw4pHo2rZrjleNFip-z_qKiQ-WP6UK80AJgvaFs"
  assert crypto.verify_signed_message(signed, <<"secret":utf8>>)
    == Ok(<<"hello":utf8>>)
  assert crypto.verify_signed_message(signed, <<"wrong":utf8>>) == Error(Nil)
  assert crypto.verify_signed_message(
      "SFMyNTY.aGVsbG8.6h6Axw4pHo2rZrjleNFip-z_qKiQ-WP6UK80AJgvaFs",
      <<"secret":utf8>>,
    )
    == Error(Nil)
  assert crypto.secure_compare(<<>>, <<>>)
  assert crypto.secure_compare(<<1, 2, 3>>, <<1, 2, 3>>)
  assert !crypto.secure_compare(<<1, 2, 3>>, <<1, 2, 4>>)
  assert !crypto.secure_compare(<<1, 2>>, <<1, 2, 3>>)
  assert bit_array.byte_size(crypto.strong_random_bytes(16)) == 16
  assert crypto.strong_random_bytes(0) == <<>>
  True
}

fn vector(algorithm, digest_hex, hmac_hex) {
  let assert Ok(expected_digest) = bit_array.base16_decode(digest_hex)
  let assert Ok(expected_hmac) = bit_array.base16_decode(hmac_hex)
  assert crypto.hash(algorithm, <<"abc":utf8>>) == expected_digest
  assert crypto.hmac(
      <<"The quick brown fox jumps over the lazy dog":utf8>>,
      algorithm,
      <<"key":utf8>>,
    )
    == expected_hmac
}

fn signed_vector(algorithm, expected) {
  let signed =
    crypto.sign_message(<<"hello":utf8>>, <<"secret":utf8>>, algorithm)
  assert signed == expected
  assert crypto.verify_signed_message(signed, <<"secret":utf8>>)
    == Ok(<<"hello":utf8>>)
}
