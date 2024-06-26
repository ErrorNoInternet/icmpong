# icmpong
The classic game of [Pong](https://en.wikipedia.org/wiki/Pong), in your terminal, over ICMPv6!

https://github.com/ErrorNoInternet/icmpong/assets/61157623/08f245c1-0167-433f-aa61-bbfdb253f1c3

### How does this work?
We are basically "using the `ping` command to send data" between two people, instead of using TCP or UDP.

ICMP allows you to include custom data (patterns) within your packets, so we're sending all of our information (ball position, scores, paddle positions, etc) through ICMP.

### Why ICMPv6 (IPv6)?
Unlike IPv4, each device gets its own **unique** IPv6 address, which means you can connect directly to your friends without any sort of intermediate server.

### How can I play?
Simply run icmpong (as root, as it needs to send and capture raw ICMP packets) and supply your friend's IPv6 address (or `fe80::101` to play with yourself) via the `-p` flag. Your friend must also supply your IPv6 address.

## Compiling
```shell
$ git clone https://github.com/ErrorNoInternet/icmpong
$ cd icmpong
$ cargo build --release
```

### Nix
```shell
$ nix run github:ErrorNoInternet/icmpong -- --help
```

(code is absolutely horrendous)

Thanks to @SpamixOfficial for the idea!
