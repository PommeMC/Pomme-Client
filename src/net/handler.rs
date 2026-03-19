use azalea_core::position::ChunkPos;
use azalea_core::registry_holder::RegistryHolder;
use azalea_protocol::packets::game::{ClientboundGamePacket, ServerboundGamePacket};
use crossbeam_channel::Sender;

use super::sender::PacketSender;
use super::NetworkEvent;

pub fn handle_game_packet(
    packet: &ClientboundGamePacket,
    sender: &PacketSender,
    event_tx: &Sender<NetworkEvent>,
    registry_holder: &RegistryHolder,
) {
    match packet {
        ClientboundGamePacket::Login(p) => {
            if let Some((_, dim)) = p.common.dimension_type(registry_holder) {
                let _ = event_tx.try_send(NetworkEvent::DimensionInfo {
                    height: dim.height,
                    min_y: dim.min_y,
                });
            }
            let _ = event_tx.try_send(NetworkEvent::GameModeChanged {
                game_mode: p.common.game_type as u8,
            });
        }
        ClientboundGamePacket::LevelChunkWithLight(p) => {
            log::trace!(
                "Chunk [{}, {}] ({} block entities)",
                p.x,
                p.z,
                p.chunk_data.block_entities.len()
            );
            let _ = event_tx.try_send(NetworkEvent::ChunkLoaded {
                pos: ChunkPos::new(p.x, p.z),
                data: p.chunk_data.data.clone(),
                heightmaps: p.chunk_data.heightmaps.clone(),
            });
        }
        ClientboundGamePacket::ForgetLevelChunk(p) => {
            let _ = event_tx.try_send(NetworkEvent::ChunkUnloaded { pos: p.pos });
        }
        ClientboundGamePacket::SetChunkCacheCenter(p) => {
            let _ = event_tx.try_send(NetworkEvent::ChunkCacheCenter { x: p.x, z: p.z });
        }
        ClientboundGamePacket::PlayerPosition(p) => {
            let _ = event_tx.try_send(NetworkEvent::PlayerPosition {
                x: p.change.pos.x,
                y: p.change.pos.y,
                z: p.change.pos.z,
                yaw: p.change.look_direction.y_rot(),
                pitch: p.change.look_direction.x_rot(),
            });
            sender.send(ServerboundGamePacket::AcceptTeleportation(
                azalea_protocol::packets::game::s_accept_teleportation::ServerboundAcceptTeleportation {
                    id: p.id,
                },
            ));
        }
        ClientboundGamePacket::KeepAlive(p) => {
            sender.send(ServerboundGamePacket::KeepAlive(
                azalea_protocol::packets::game::s_keep_alive::ServerboundKeepAlive { id: p.id },
            ));
        }
        ClientboundGamePacket::ChunkBatchFinished(p) => {
            let desired = (p.batch_size as f32).max(25.0);
            log::trace!(
                "ChunkBatchFinished: batch_size={}, responding with desired={desired}",
                p.batch_size
            );
            sender.send(ServerboundGamePacket::ChunkBatchReceived(
                azalea_protocol::packets::game::s_chunk_batch_received::ServerboundChunkBatchReceived {
                    desired_chunks_per_tick: desired,
                },
            ));
        }
        ClientboundGamePacket::ContainerSetContent(p) if p.container_id == 0 => {
            let _ = event_tx.try_send(NetworkEvent::InventoryContent {
                items: p.items.clone(),
            });
        }
        ClientboundGamePacket::ContainerSetSlot(p)
            if p.container_id == 0 || p.container_id == -2 =>
        {
            let _ = event_tx.try_send(NetworkEvent::InventorySlot {
                index: p.slot,
                item: p.item_stack.clone(),
            });
        }
        ClientboundGamePacket::SetHealth(p) => {
            let _ = event_tx.try_send(NetworkEvent::PlayerHealth {
                health: p.health,
                food: p.food,
                saturation: p.saturation,
            });
        }
        ClientboundGamePacket::SystemChat(p) if !p.overlay => {
            send_chat(event_tx, p.content.to_string());
        }
        ClientboundGamePacket::PlayerChat(p) => {
            send_chat(event_tx, p.message().to_string());
        }
        ClientboundGamePacket::DisguisedChat(p) => {
            send_chat(event_tx, p.message.to_string());
        }
        ClientboundGamePacket::BlockUpdate(p) => {
            let _ = event_tx.try_send(NetworkEvent::BlockUpdate {
                pos: p.pos,
                state: p.block_state,
            });
        }
        ClientboundGamePacket::SectionBlocksUpdate(p) => {
            let updates: Vec<_> = p
                .states
                .iter()
                .map(|s| {
                    let block_pos = azalea_core::position::BlockPos {
                        x: p.section_pos.x * 16 + s.pos.x as i32,
                        y: p.section_pos.y * 16 + s.pos.y as i32,
                        z: p.section_pos.z * 16 + s.pos.z as i32,
                    };
                    (block_pos, s.state)
                })
                .collect();
            let _ = event_tx.try_send(NetworkEvent::SectionBlocksUpdate { updates });
        }
        ClientboundGamePacket::BlockChangedAck(p) => {
            let _ = event_tx.try_send(NetworkEvent::BlockChangedAck { seq: p.seq });
        }
        ClientboundGamePacket::SetTime(p) => {
            let _ = event_tx.try_send(NetworkEvent::TimeUpdate {
                game_time: p.game_time,
                day_time: p.day_time,
            });
        }
        ClientboundGamePacket::SetChunkCacheRadius(p) => {
            let _ = event_tx.try_send(NetworkEvent::ServerViewDistance { distance: p.radius });
        }
        ClientboundGamePacket::SetSimulationDistance(p) => {
            let _ = event_tx.try_send(NetworkEvent::ServerSimulationDistance {
                distance: p.simulation_distance,
            });
        }
        ClientboundGamePacket::GameEvent(p) => {
            use azalea_protocol::packets::game::c_game_event::EventType;
            if p.event == EventType::ChangeGameMode {
                let _ = event_tx.try_send(NetworkEvent::GameModeChanged {
                    game_mode: p.param as u8,
                });
            }
        }
        ClientboundGamePacket::Disconnect(p) => {
            log::warn!("Disconnected: {}", p.reason);
            let _ = event_tx.try_send(NetworkEvent::Disconnected {
                reason: format!("{}", p.reason),
            });
        }
        _other => {}
    }
}

fn send_chat(event_tx: &Sender<NetworkEvent>, text: String) {
    log::info!("Chat: {text}");
    let _ = event_tx.try_send(NetworkEvent::ChatMessage { text });
}
