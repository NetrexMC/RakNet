# Reliability

Reliability within RakNet specifies when a packet should arrive and how it should be treated. The term "Reliability" within RakNet generally refers to the following possible states of a packet:

- **Unreliable**. The packet sent is not reliable and not important. Later packets will make up for it's loss. This is the same as a regular UDP packet with the added benefit that duplicates are discarded.

- **UnreliableSequenced**. This packet is not necessarily important, if it arrives later than another packet, you should discard it. Otherwise you can treat it normally. *This reliability is generally not used*

- **Reliable**. The packet should be acknow
