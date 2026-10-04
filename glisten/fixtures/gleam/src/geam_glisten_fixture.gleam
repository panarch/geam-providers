import argv
import glisten_alpn_contract
import glisten_server_contract
import glisten_supervised_contract
import glisten_tcp_contract
import glisten_tls_contract
import glisten_user_contract

pub fn verify(certfile: String, keyfile: String) -> Nil {
  glisten_tcp_contract.main()
  glisten_server_contract.main()
  glisten_user_contract.main()
  glisten_supervised_contract.main()
  glisten_tls_contract.verify(certfile, keyfile)
  glisten_alpn_contract.verify(certfile, keyfile)
}

pub fn main() {
  let assert [certfile, keyfile] = argv.load().arguments
  verify(certfile, keyfile)
}
