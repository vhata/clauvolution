use bevy::prelude::*;
use clauvolution_genome::{Genome, SegmentType, Symmetry};

pub struct BodyPlugin;

impl Plugin for BodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_body_plans);
    }
}

/// A rendered body part with position relative to organism center
#[derive(Clone, Debug)]
pub struct RenderedPart {
    pub segment_type: SegmentType,
    pub offset: Vec2,
    pub size: f32,
    pub angle: f32,
}

/// The decoded body plan, ready for rendering
#[derive(Component, Clone, Debug)]
pub struct BodyPlan {
    pub parts: Vec<RenderedPart>,
    pub total_size: f32,
}

impl BodyPlan {
    pub fn from_genome(genome: &Genome) -> Self {
        let mut parts = Vec::new();
        let base_scale = genome.body_size;

        for (i, seg) in genome.body_segments.iter().enumerate() {
            if i == 0 {
                // Torso at center
                parts.push(RenderedPart {
                    segment_type: seg.segment_type,
                    offset: Vec2::ZERO,
                    size: seg.size * base_scale,
                    angle: 0.0,
                });
            } else {
                // Attached parts radiate from torso
                let angle = seg.attachment_angle;
                let dist = genome.body_segments[0].size * base_scale * 0.6;
                let offset = Vec2::new(angle.cos() * dist, angle.sin() * dist);

                parts.push(RenderedPart {
                    segment_type: seg.segment_type,
                    offset,
                    size: seg.size * base_scale * 0.6,
                    angle,
                });

                // Bilateral symmetry: mirror across y-axis
                if seg.symmetry == Symmetry::Bilateral {
                    parts.push(RenderedPart {
                        segment_type: seg.segment_type,
                        offset: Vec2::new(-offset.x, offset.y),
                        size: seg.size * base_scale * 0.6,
                        angle: std::f32::consts::PI - angle,
                    });
                }
            }
        }

        let total_size = parts.iter().map(|p| p.size).sum::<f32>().max(0.5);

        BodyPlan { parts, total_size }
    }
}

/// System that creates/updates body plans from genomes
fn update_body_plans(
    mut commands: Commands,
    query: Query<(Entity, &Genome), Without<BodyPlan>>,
) {
    for (entity, genome) in &query {
        let body_plan = BodyPlan::from_genome(genome);
        commands.entity(entity).insert(body_plan);
    }
}
