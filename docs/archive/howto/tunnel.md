# Tunnel Guide

The zero-trust tunnel eliminates physical distance limitations, allowing you to access your devices from anywhere. For example, you can run zt-tunnel on your home computer and access it from your office computer.

Use the command `ztm tunnel help` to view the usage of `tunnel`.

> Note that the first time you operate the app (including `help`), it will start the application.

```sh
ztm tunnel help
Commands:

  ztm tunnel get       <object type>                 List objects of the specified type
  ztm tunnel describe  <object type> <object name>   Show detailed info of the specified object
  ztm tunnel open      <object type> <object name>   Create an object of the specified type
  ztm tunnel close     <object type> <object name>   Delete the specified object

Object Types:

  inbound  in    Inbound end of a tunnel
  outbound out   Outbound end of a tunnel

Object Names:

  tcp/<name>     Name for a TCP tunnel
  udp/<name>     Name for a UDP tunnel

Type 'ztm tunnel help <command>' for detailed info.
```

In `zt-tunnel`, there are two resources: `inbound` and `outbound`, corresponding to the tunnel's entrance and exit, respectively. These can connect devices located in different networks.

Assume we have two Agents in different networks: `root` and `test`. In the `test` network, there are two web services on ports `8080` and `8081`.

```sh
curl 198.19.249.3:8080
hi
curl 198.19.249.3:8081
hello
```

Typically, to access services in another network, we need to open ports in the firewall and use a public IP address. However, with ZTM, this becomes much simpler.

First, we open an `outbound` on the `test` Agent, named *tcp/greeting*, and specify the two web services as its *targets*.

```sh
# agent test 
ztm tunnel open outbound tcp/greeting --targets 198.19.249.3:8080 --targets 198.19.249.3:8081
ztm tunnel get outbound

NAME          TARGETS                               ENTRANCES
tcp/greeting  198.19.249.3:8080, 198.19.249.3:8081
```

After opening the tunnel's exit, we open the `inbound` on the `root` Agent side, also naming it *tcp/greeting*.

```sh
# agent root 
ztm tunnel open inbound tcp/greeting --listen 18080
```

We can request `localhost:18080` multiple times to get responses from both *targets*.

```sh
curl localhost:18080
hi
curl localhost:18080
hello
curl localhost:18080
hi
```
