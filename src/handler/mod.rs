use std::collections::{VecDeque};
use binary_utils::{BinaryStream, IBinaryStream, IBufferRead, IBufferWrite};
use crate::{IServerBound, IClientBound};
use crate::ack::{Ack, Record, SingleRecord};
use crate::frame::{Frame, FramePacket};
use crate::fragment::{Fragment, FragmentInfo, FragmentList, FragmentStore};
use crate::reliability::{Reliability, ReliabilityFlag};
use crate::conn::{Connection};
use crate::protocol::offline::*;
use crate::online::{handle_online, OnlinePackets};
use crate::ack::is_ack_or_nack;

/// PacketHandler handles all packets, both outbound and inbound.
#[derive(Clone)]
pub struct PacketHandler {
     /// A Vector of streams to be sent.
     /// This should almost always be a Frame, with exceptions
     /// to offline packets.
     pub send_queue: VecDeque<BinaryStream>,
     /// Stores the fragmented frames by their
     /// `frame_index` value from a given packet.
     /// When a `FrameList` is ready from a `FragmentStore` it's assembled
     /// into a `FramePacket` which can then be added to the `send_queue`.
     fragmented: FragmentStore,
     /// Stores the next available fragment id.
     /// This variable will reset after the sequence
     /// containing the fragment id's we sent has been
     /// acknowledged by the client.
     ///
     /// However in the event this never occurs, fragment id will reset after
     /// it reaches `65535` as a value
     fragment_id: u16,
     recv_seq: u32,
     send_seq: u32,
     //ack: ReliableQueue,
     //nack: ReliableQueue
}

impl PacketHandler {
     pub fn new() -> Self {
          Self {
               send_queue: VecDeque::new(),
               fragmented: FragmentStore::new(),
               recv_seq: 0,
               send_seq: 0,
               fragment_id: 0
          }
     }

     pub fn recv(&mut self, connection: &mut Connection, stream: &mut BinaryStream) {
          if !connection.connected {
               let pk = OfflinePackets::recv(stream.read_byte());
               let handler = handle_offline(connection, pk, stream);
               self.send_queue.push_back(handler);
          } else {
               // this packet is almost always a frame packet
               let online_packet = OnlinePackets::recv(stream.read_byte());

               if is_ack_or_nack(online_packet.to_byte()) {
                    return;
               }

               if !match online_packet { OnlinePackets::FramePacket(_) => true, _ => false } {
                    return;
               }

               let mut frame_packet = FramePacket::recv(stream.clone());

               // todo Handle ack and nack!
               self.handle_frames(connection, &mut frame_packet);
          }
     }

     pub fn handle_frames(&mut self, connection: &mut Connection, frame_packet: &mut FramePacket) {
          for frame in frame_packet.frames.iter_mut() {
               if frame.fragment_info.is_some() {
                    // the frame is fragmented!
                    self.fragmented.add_frame(frame.clone());

                    if self.fragmented.ready(frame.fragment_info.unwrap().fragment_index as u16) {
                         let pk = self.fragmented.assemble_frame(frame.fragment_info.unwrap().fragment_index as u16, connection.mtu_size as i16, self.fragment_id);
                         if pk.is_some() {
                              for f in pk.unwrap().frames.iter_mut() {
                                   self.handle_full_frame(connection, f);
                              }
                              self.fragment_id += 1;
                         }
                    } else {
                         println!(
                              "Frame: {} is not ready! It needs {} more fragments!",
                              frame.fragment_info.unwrap().fragment_id,
                              self.fragmented.fragment_table.get(&frame.fragment_info.unwrap().fragment_id).unwrap().get_remaining_size()
                         );
                    }

                    continue;
               }

               self.handle_full_frame(connection, frame);
          }
     }

     fn handle_full_frame(&mut self, connection: &mut Connection, frame: &mut Frame) {
          // todo Check if the frames should be recieved, if not purge them
          // todo EG: add implementation for ordering and sequenced frames!
          let online_packet = OnlinePackets::recv(frame.body.read_byte());

          println!("\nPacket Recieved: {:?}", online_packet);

          if online_packet == OnlinePackets::GamePacket {
               // todo add a game packet handler for invokation
               // todo probably make this a box to a fn
               println!("Got a game packet.");
          } else {
               let mut response = handle_online(connection, online_packet.clone(), &mut frame.body);

               if response.get_length() != 0 {
                    if response.get_length() as u16 > connection.mtu_size {
                         self.fragment(connection, &mut response)
                    } else {
                         let mut new_framepk = FramePacket::new();
                         let mut new_frame = Frame::init();

                         new_frame.body = response.clone();
                         new_frame.reliability = Reliability::new(ReliabilityFlag::Unreliable);
                         new_framepk.frames.push(new_frame);
                         new_framepk.seq = self.send_seq;
                         self.send_queue.push_back(new_framepk.to());
                         self.send_seq = self.send_seq + 1;
                    }
               }
               // println!("\nSent: {:?}", response.clone());
               // self.send_queue.push_back(response);
          }
     }

     /// Automatically fragment the stream based on the clients mtu
     /// size and add the frames to the handler queue.
     /// todo FIX THIS
     pub fn fragment(&mut self, connection: &mut Connection, stream: &mut BinaryStream) {
          let usable_id = self.fragment_id + 1;

          if usable_id == 65535 {
               self.fragment_id = 0;
          }

          let mut fragment_list = FragmentList::new();
          let mut index: i32 = 0;
          let mut offset: usize = stream.get_length();

          loop {
               if offset == 0 {
                    break;
               }

               let mut next = BinaryStream::init(&stream.get_buffer());

               if stream.get_length() > connection.mtu_size as usize {
                    next = stream.slice(0, Some(connection.mtu_size as usize));
                    offset -= connection.mtu_size as usize;
               } else {
                    offset -= stream.get_length();
               }

               let frag = Fragment::new(index as i32, next.get_buffer());

               fragment_list.add_fragment(frag);
               index += 1;
          }
          println!("List: {:?}", fragment_list);

          let packets = fragment_list.assemble(connection.mtu_size as i16, usable_id);
          println!("Packet: {:?}", packets);
          if packets.is_some() {
               for packet in packets.unwrap().iter_mut() {
                    packet.seq = self.send_seq + 1;

                    self.send_queue.push_back(packet.to());
               }
          }

          self.fragment_id += 1;
     }
}