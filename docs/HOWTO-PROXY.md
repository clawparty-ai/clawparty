# Proxy Guide

The zero-trust proxy allows you to access your devices from anywhere. For example, you can run zt-proxy on your home computer and access all your devices from your office computer as if they were on the same local network.

zt-proxy is divided into a listening side and a forwarding side. The listening side receives requests and transmits them through a secure encrypted channel to the forwarding side, which forwards the requests to the target server.

```sh
ztm proxy help config

Configure an endpoint as a listening side and/or a forwarding side

Usage: ztm proxy config [options...]

Options:

  --set-listen    [[ip:]port]       Open/close the proxy port
  --add-target    <domain|ip ...>   Add targets where traffic leaving from the endpoint can go
                                    e.g. '*.example.com' and '8.88.0.0/16'
  --remove-target <domain|ip ...>   Remove previously added targets
```

Assume we have two Agents in different networks: `root` and `test`. The latter acts as the forwarding side. First, we set forwarding rules on the `test` Agent: only forwarding requests matching the rules.

```sh
ztm get ep
NAME          USER  IP             PORT   STATUS
ep-1 (local)  root  103.116.72.12  13297  Online
ep-2          test  45.62.167.230  18687  Online
```

On the forwarding side, we set forwarding rules for `0.0.0.0/0` and `*`, meaning forwarding requests for all network segments and hosts.

```sh
# agent test
ztm proxy config --add-target 0.0.0.0/0 '*'

Endpoint: ep-2 (af2fc697-ff1e-4e44-9e46-b12e5a5c818e)
Listen: (not listening)
Targets:
  0.0.0.0/0
  *
```

Next, configure the listening side. The listening side receives requests and needs to be configured with a listening address and port.

```sh
# agent root
ztm proxy config --set-listen 0.0.0.0:1080
```

With zt-proxy configured, we can test it. The `test` Agent has a web service listening on `127.0.1.1:8082`, which is only locally accessible. The domain name `ubuntu` resolves to this address.

```sh
# agent test
cat /etc/hosts
127.0.1.1 ubuntu
127.0.0.1 localhost
...
```

Using zt-proxy, we can successfully access it through the proxy on the `root` Agent side.

```sh
# agent root
curl localhost:8082
curl: (7) Failed to connect to localhost port 8082 after 4 ms: Connection refused

curl --proxy http://localhost:1080 localhost:8082
Hi, there!
```

Similarly, using the domain name, we can also access the web service.

```sh
# agent root
curl ubuntu:8082
curl: (6) Could not resolve host: ubuntu
curl --proxy http://localhost:1080 ubuntu:8082
Hi, there!
```
