# Fixture TLS credentials

This self-signed localhost certificate and its public test private key are only
for the mandatory loopback fixtures. The certificate includes `localhost` and
`127.0.0.1` subject alternative names and is valid from 2020-01-01 through
2100-01-01. Production callers supply their own certificate and private-key
paths through the original Glisten TLS options.
