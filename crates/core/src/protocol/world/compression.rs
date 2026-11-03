use anyhow::{Result, ensure};
use bytes::{BufMut, Bytes, BytesMut};
use flate2::{
    Compress, Compression, Decompress, FlushCompress,
    FlushDecompress, Status,
};

use super::MAX_BODY_SIZE;

const ADLER_SEED: u32 = 0x9827_d8f1;

fn adler32(data: &[u8]) -> u32 {
    let mut low = ADLER_SEED & 0xffff;
    let mut high = ADLER_SEED >> 16;

    for block in data.chunks(5552) {
        for byte in block {
            low += u32::from(*byte);
            high += low;
        }

        low %= 65_521;
        high %= 65_521;
    }

    (high << 16) | low
}

pub(super) struct Inflater {
    stream: Decompress,
}

impl Default for Inflater {
    fn default() -> Self {
        Self {
            stream: Decompress::new(false),
        }
    }
}

impl Inflater {
    pub fn decode(&mut self, payload: &[u8]) -> Result<Bytes> {
        ensure!(payload.len() >= 12, "Short World compression header");

        let expected =
            u32::from_le_bytes(payload[..4].try_into()?) as usize;

        let plain_sum =
            u32::from_le_bytes(payload[4..8].try_into()?);

        let packed_sum =
            u32::from_le_bytes(payload[8..12].try_into()?);

        let packed = &payload[12..];

        ensure!(
            (4..=MAX_BODY_SIZE).contains(&expected),
            "Invalid World decompressed size: {expected}",
        );

        ensure!(
            adler32(packed) == packed_sum,
            "World compressed checksum mismatch",
        );

        let mut output = vec![0u8; expected + 1];

        let before_in = self.stream.total_in();
        let before_out = self.stream.total_out();

        let status = self.stream.decompress(
            packed,
            &mut output,
            FlushDecompress::Sync,
        )?;

        let consumed =
            (self.stream.total_in() - before_in) as usize;

        let produced =
            (self.stream.total_out() - before_out) as usize;

        ensure!(
            status != Status::StreamEnd,
            "World DEFLATE stream ended unexpectedly",
        );

        ensure!(
            consumed == packed.len() && produced == expected,
            "Invalid World decompression: \
             consumed={consumed}/{}, produced={produced}/{expected}",
            packed.len(),
        );

        output.truncate(produced);

        ensure!(
            adler32(&output) == plain_sum,
            "World decompressed checksum mismatch",
        );

        Ok(Bytes::from(output))
    }
}

pub(super) struct Deflater {
    stream: Compress,
}

impl Default for Deflater {
    fn default() -> Self {
        Self {
            stream: Compress::new(Compression::default(), false),
        }
    }
}

impl Deflater {
    pub fn encode(&mut self, body: &[u8]) -> Result<Bytes> {
        ensure!(
            (4..=MAX_BODY_SIZE).contains(&body.len()),
            "Invalid World body size for compression",
        );

        let mut packed = Vec::new();
        let mut offset = 0usize;
        let mut buffer = [0u8; 8192];

        loop {
            let before_in = self.stream.total_in();
            let before_out = self.stream.total_out();

            let status = self.stream.compress(
                &body[offset..],
                &mut buffer,
                FlushCompress::Sync,
            )?;

            let consumed =
                (self.stream.total_in() - before_in) as usize;

            let produced =
                (self.stream.total_out() - before_out) as usize;

            offset += consumed;
            packed.extend_from_slice(&buffer[..produced]);

            ensure!(
                packed.len() <= MAX_BODY_SIZE,
                "Compressed World body exceeds limit",
            );

            ensure!(
                status != Status::StreamEnd,
                "World compressor ended unexpectedly",
            );

            if offset == body.len() && produced < buffer.len() {
                break;
            }

            ensure!(
                consumed != 0 || produced != 0,
                "World compressor made no progress",
            );
        }

        let mut payload = BytesMut::with_capacity(12 + packed.len());

        payload.put_u32_le(body.len() as u32);
        payload.put_u32_le(adler32(body));
        payload.put_u32_le(adler32(&packed));
        payload.extend_from_slice(&packed);

        Ok(payload.freeze())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(
                    std::str::from_utf8(pair).unwrap(),
                    16,
                )
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn reads_python_stream_with_persistent_dictionary() {
        let first = unhex(
            "1400000066dadba1c1eaf3aadacee8cac0c0c8c4ccc2cac6cec1c9c5cdc3cbc70f000000ffff",
        );

        let second = unhex(
            "1400000066dadba1a8dc5a4ada8e450c000000ffff",
        );

        let expected =
            unhex("b7014500000102030405060708090a0b0c0d0e0f");

        let mut inflater = Inflater::default();

        assert_eq!(&inflater.decode(&first).unwrap()[..], expected);
        assert_eq!(&inflater.decode(&second).unwrap()[..], expected);
    }

    #[test]
    fn round_trips_multiple_compressed_packets() {
        let body =
            unhex("b7014500000102030405060708090a0b0c0d0e0f");

        let mut encoder = Deflater::default();
        let mut decoder = Inflater::default();

        for _ in 0..3 {
            let compressed = encoder.encode(&body).unwrap();
            assert_eq!(&decoder.decode(&compressed).unwrap()[..], body);
        }
    }
}
