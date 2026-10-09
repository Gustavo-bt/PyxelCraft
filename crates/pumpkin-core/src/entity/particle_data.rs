use pumpkin_data::Block;
use pumpkin_data::particle::Particle;

pub fn build_particle_data(particle: Particle) -> Vec<u8> {
    let mut data = Vec::new();
    match particle {
        Particle::Block | Particle::BlockMarker | Particle::BlockCrumble
        | Particle::FallingDust | Particle::DustPillar => {
            let state_id = i32::from(Block::STONE.default_state.id.as_u16());
            write_varint(&mut data, state_id);
        }
        Particle::Dust => {
            for v in [1.0f32, 0.0, 0.0, 1.0] {
                data.extend_from_slice(&v.to_be_bytes());
            }
        }
        Particle::DustColorTransition => {
            for _ in 0..8 {
                data.extend_from_slice(&1.0f32.to_be_bytes());
            }
        }
        Particle::EntityEffect => {
            data.extend_from_slice(&0xFFFF_FFFFu32.to_be_bytes());
        }
        Particle::SculkCharge => {
            data.extend_from_slice(&0.0f32.to_be_bytes());
        }
        Particle::Shriek | Particle::Note => {
            write_varint(&mut data, 0);
        }
        _ => {}
    }
    data
}

fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        if (v & !0x7F) == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push(((v & 0x7F) | 0x80) as u8);
        v >>= 7;
    }
}
