# Controller bridge protocol

The controller bridge exposes the controller used by CNC_Lab to a local
process. It listens on the Unix-domain socket
`/run/pimprobe-controller.sock`. The socket path can be changed by the client,
but the proxy plugin always uses this path.

The bridge accepts one client at a time. A connection attempted while another
client is connected is closed immediately. Clients should reconnect after the
existing connection ends or when the socket is temporarily unavailable.

## Frames

Communication is a stream of length-prefixed frames. Integers are unsigned and
use network byte order (big endian).

| Offset | Size | Description |
| ---: | ---: | --- |
| 0 | 1 byte | Frame type |
| 1 | 4 bytes | Payload length |
| 5 | payload length | Payload |

For example, a frame with type `Q` and the four-byte payload `G54\n` is:

```text
51 00 00 00 04 47 35 34 0a
```

The socket is a byte stream. A read may contain part of a frame, one frame, or
several frames. Receivers must use the length field rather than treating socket
reads as message boundaries.

## Frame types

| Type | Direction | Meaning |
| --- | --- | --- |
| `Q` | Client to bridge | Submit a command through CNC_Lab's normal command queue. |
| `R` | Client to bridge | Submit bytes through CNC_Lab's realtime-command path. |
| `D` | Bridge to client | Controller output. The payload may contain one record, part of a record, or several newline-separated records. |

The bridge treats payloads as opaque bytes. It does not interpret controller
commands or responses.

Commands sent in a `Q` frame must include any terminator required by the
controller. Realtime payloads are forwarded exactly as received and do not
need a line ending.

## Limits

| Item | Limit |
| --- | ---: |
| Any frame | 1,048,576 bytes |
| `Q` payload | 4,096 bytes |
| `R` payload | 64 bytes |
| Pending output to a client | 262,144 bytes |

The bridge closes the connection when it receives an unknown frame type, an
oversized frame or an input frame whose type-specific limit is exceeded. It
also closes a client that lets pending output exceed the limit or when a socket
write cannot accept the complete frame.

An incomplete frame remains buffered until the rest arrives. There is no
request identifier and no one-to-one pairing between input and output frames.
Controller output is asynchronous, so clients must parse the records carried
by `D` frames rather than assume that the next frame is the reply to the most
recent command.

## Output sources

The proxy forwards CNC_Lab's parsed controller records. When the installed
CNC_Lab version also exposes raw controller data, the proxy extracts printable
status records containing `|MPos:` and forwards those as well. This keeps the
wire contract the same across supported CNC_Lab versions; clients may receive
different subsets of controller records depending on the signals available in
the host application.

