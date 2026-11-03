use std::collections::HashMap;

use anyhow::Result;

use crate::handlers::Direction;
use crate::protocol::world::WorldPacket;

use super::FeatureConfig;
use super::world_time::WorldTimeFeature;

pub(super) trait WorldFeature: Send {
    fn direction(&self) -> Direction;

    fn opcodes(&self) -> &'static [u32];

    fn apply(&mut self, packet: &mut WorldPacket) -> Result<()>;
}

pub(crate) struct WorldPipeline {
    features: Vec<Box<dyn WorldFeature>>,
    routes: HashMap<(Direction, u32), Vec<usize>>,
}

impl WorldPipeline {
    pub(crate) fn new(config: &FeatureConfig) -> Result<Self> {
        config.validate()?;

        let mut pipeline = Self {
            features: Vec::new(),
            routes: HashMap::new(),
        };

        // Registration order defines execution order for shared opcodes.
        if let Some(settings) = &config.world_time {
            pipeline.register(Box::new(WorldTimeFeature::new(settings.clone())?));
        }

        Ok(pipeline)
    }

    fn register(&mut self, feature: Box<dyn WorldFeature>) {
        let index = self.features.len();
        let direction = feature.direction();

        for &opcode in feature.opcodes() {
            let route = self.routes.entry((direction, opcode)).or_default();

            if !route.contains(&index) {
                route.push(index);
            }
        }

        self.features.push(feature);
    }

    pub(crate) fn process(
        &mut self,
        direction: Direction,
        packets: &[WorldPacket],
    ) -> Result<Option<Vec<WorldPacket>>> {
        if self.routes.is_empty()
            || !packets
                .iter()
                .any(|packet| self.routes.contains_key(&(direction, packet.opcode)))
        {
            return Ok(None);
        }

        let mut output = packets.to_vec();

        for packet in &mut output {
            // Route by the original opcode; do not recursively redispatch.
            let Some(indices) = self.routes.get(&(direction, packet.opcode)) else {
                continue;
            };

            for &index in indices {
                self.features[index].apply(packet)?;
            }
        }

        Ok(Some(output))
    }
}
