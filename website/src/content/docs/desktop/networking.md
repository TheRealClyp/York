---
title: TCP Networking
description: Real-time socket programming in York.
---

York provides built-in TCP networking primitives (`net_*`) for building high-performance servers and clients.

### Example: TCP Echo Server

```java
public static void main(String[] args) {
    long server = net_listen(8080);
    println("TCP Server listening on port 8080...");
    
    while (true) {
        long client = net_accept(server);
        if (client >= 0) {
            String msg = net_recv(client);
            println("Received: " + msg);
            net_send(client, "HTTP/1.1 200 OK\r\n\r\nHello from York!");
            net_close(client);
        }
    }
}
```

### Networking Functions
- `net_listen(port)`
- `net_accept(server_sock)`
- `net_connect(host, port)`
- `net_send(sock, data)`
- `net_recv(sock)`
- `net_close(sock)`
