## RakNet

RakNet doesn't have a lot of resources on it's internal functionality, and to better understand it I've written these docs.

RakNet is composed and derived of multiple parts, so for simplicity I've narrowed it down into the following parts:

- [Protocol](./protocol/README.md)
  
  - [Connection Handshake](./protocol/handshake.md)
  
  - [Datagrams](./protocol/datagrams)

- [Packet Reliability](./reliability.md)

- [Congestion Control](./congestion.md)

- [Plugins]()

- [Server]()

- [Client]()

It's important to note that this documentation does conceptualize and take a different approach from the original RakNet implementation, however it does not deviate from the behavior of RakNet; rather providing a basis on how I believe it should be implemented.
