# SOCKS5 proxy test certificates

Used by `tests/socks5_proxy.rs`. A throwaway CA (its key was discarded) and a
server certificate for `IP:127.0.0.1`, `IP:::1` and `DNS:localhost`, both
valid for 100 years. `server.key` is the server's test-only private key.

Regenerate with:

```sh
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 36500 \
  -subj "/CN=rs-dapi-client test CA" -keyout ca.key -out ca.pem
openssl req -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes \
  -subj "/CN=localhost" -keyout server.key.sec1 -out server.csr
printf 'subjectAltName=IP:127.0.0.1,IP:::1,DNS:localhost\nbasicConstraints=CA:FALSE\nextendedKeyUsage=serverAuth\n' > ext.cnf
openssl x509 -req -in server.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -days 36500 -extfile ext.cnf -out server.pem
openssl pkcs8 -topk8 -nocrypt -in server.key.sec1 -out server.key
rm ca.key ca.srl server.csr server.key.sec1 ext.cnf
```
