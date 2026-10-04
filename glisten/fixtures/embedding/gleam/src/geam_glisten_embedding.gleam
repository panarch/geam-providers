import geam_glisten_fixture

pub fn verify(certfile: String, keyfile: String) -> Nil {
  geam_glisten_fixture.verify(certfile, keyfile)
}
