use super::*;

use crate::clients::ClientId;

fn push_packet(
    codec: &mut WorldCodec,
    from_client: bool,
    packet: WorldPacket,
) -> DecodedFrame {
    let wire = encode_frame(
        &[0; 12],
        &packet.to_body().unwrap(),
    )
        .unwrap();

    codec.push(from_client, &wire);

    match codec.next(from_client).unwrap().unwrap() {
        WorldEvent::Packet(frame) => frame,
        _ => panic!("expected frame"),
    }
}

#[test]
fn remapped_auth_and_container_opcodes_reach_the_shared_codec() {
    let mut protocol = ClientId::Retail.profile().world.unwrap();

    protocol.opcodes.auth_challenge = 0x201;
    protocol.opcodes.auth_session = 0x202;
    protocol.opcodes.multiple = 0x203;
    protocol.opcodes.compressed = 0x204;

    let mut codec = WorldCodec::new(protocol);

    for (from_client, greeting) in [
        (
            false,
            b"WORLD OF WARCRAFT CONNECTION - SERVER TO CLIENT - V2\n"
                .as_slice(),
        ),
        (
            true,
            b"WORLD OF WARCRAFT CONNECTION - CLIENT TO SERVER - V2\n"
                .as_slice(),
        ),
    ] {
        codec.push(from_client, greeting);

        assert!(matches!(
            codec.next(from_client).unwrap(),
            Some(WorldEvent::Greeting(_)),
        ));
    }

    let mut challenge = vec![0; 65];

    challenge[32..64].fill(7);

    push_packet(
        &mut codec,
        false,
        WorldPacket {
            opcode: protocol.opcodes.auth_challenge,
            payload: challenge.into(),
        },
    );

    let mut auth = vec![0; 76];

    auth[20..52].fill(9);

    let frame = push_packet(
        &mut codec,
        true,
        WorldPacket {
            opcode: protocol.opcodes.auth_session,
            payload: auth.into(),
        },
    );

    match frame.auth.unwrap() {
        AuthRequest::Initial { server, client } => {
            assert_eq!(server, [7; 32]);
            assert_eq!(client, [9; 32]);
        }

        _ => panic!("expected initial authentication"),
    }

    let messages = vec![WorldPacket {
        opcode: 0x301,
        payload: Bytes::from_static(b"data"),
    }];

    let body = bundle_messages(
        &messages,
        true,
        protocol.opcodes.multiple,
    )
        .unwrap();

    let compressed = Deflater::default()
        .encode(&body)
        .unwrap();

    let frame = push_packet(
        &mut codec,
        false,
        WorldPacket {
            opcode: protocol.opcodes.compressed,
            payload: compressed,
        },
    );

    assert!(frame.compressed);
    assert_eq!(frame.packets, messages);
}
