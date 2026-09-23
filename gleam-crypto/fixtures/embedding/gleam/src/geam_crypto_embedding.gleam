import geam_crypto_fixture
import gleam/crypto

pub fn verify() -> Bool {
  geam_crypto_fixture.verify()
}

pub fn random_four() -> BitArray {
  crypto.strong_random_bytes(4)
}

pub fn random_negative() -> BitArray {
  crypto.strong_random_bytes(-1)
}

pub fn partial_hash() -> BitArray {
  crypto.hash(crypto.Sha256, <<1:size(1)>>)
}

pub fn partial_hmac_data() -> BitArray {
  crypto.hmac(<<1:size(1)>>, crypto.Sha256, <<"key":utf8>>)
}

pub fn partial_hmac_key() -> BitArray {
  crypto.hmac(<<"data":utf8>>, crypto.Sha256, <<1:size(1)>>)
}
