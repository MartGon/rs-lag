# rs-lag

## Description

This program works as an UDP proxy to simulate network conditions. Different network parameters, such as RTT, jitter, packet loss, packet duplication and packet unordering can be configured through its command line interface.

# ![rs-lag.png](https://raw.githubusercontent.com/MartGon/rs-lag/main/docs/pictures/rs-lag.png)

## How to use

### Launching

When launching rs-lag, you can provide the following parameters:

- Bind Port (-b, --bind-port): This is the port where the proxy server will listen to connection requests. Clients should connect to this port.
- Connect Address (-c, --connect-addr): Address where connection requests will be redirected to. This should be the *server's* address.
- Lag (-l, --lag): Amount of lag in ms introduced by the network. This increases RTT by the same amount
- Jitter(-j, --jitter): Amount of jitter in ms introduced by the network. This increases RTT on Uniform(-jitter, jitter)
- Client-Server Duplication (-d, --client-server-duplication): Chance to duplicate a packet on the way from the client to the server. A float value from 0 to 100
- Server-Client Duplication (-D, --server-client-duplication): Chance to duplicate a packet on the way from the server to the client. A float value from 0 to 100
- Client-Server Unorder (-u, --client-server-unorder): Chance for a given packet to arrive after the next on the way from the client to the server. A float value from 0 to 100
- Server-Client Unorder (-U, --server-client-unorder): Chance for a given packet to arrive after the next on the way from the server to the client. A float value from 0 to 100
- Client-Server Loss (-m, --client-server-loss): Chance for a packet to be lost on the way from the client to the server. A float value from 0 to 100
- Server-Client Loss (-M, --server-client-loss): Chance for a packet to be lost on the way from the server to the client. A float value from 0 to 100
- Timeout (-t, --timeout): Time in ms (int) without network traffic to consider that a client disconnected

## About

My first project made in Rust.
