use common::{comp::HitFlash, resources::DeltaTime};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, Read, WriteStorage};

/// System that ticks down HitFlash timers and removes expired flashes.
/// HitFlash components are inserted externally (e.g. in session handle_outcome).
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, HitFlash>,
    );

    const NAME: &'static str = "hit_flash";
    const ORIGIN: Origin = Origin::Frontend("voxygen");
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut Job<Self>, (entities, dt, mut hit_flashes): Self::SystemData) {
        let dt = dt.0;

        // Tick down existing HitFlash components and remove expired ones
        let expired: Vec<_> = (&entities, &mut hit_flashes)
            .join()
            .filter_map(|(entity, flash)| {
                flash.timer -= dt;
                if flash.timer <= 0.0 {
                    Some(entity)
                } else {
                    None
                }
            })
            .collect();

        for entity in expired {
            hit_flashes.remove(entity);
        }
    }
}
