use crate::traits::*;
use glam::I16Vec2;

fn check_physical_collision<E1, E2>(one: &E1, other: &E2) -> Option<(ColliderType, ColliderType)>
where
    E1: Entity,
    E2: Entity,
{
    if one.previous_position() == one.position() {
        return None;
    }
    // Find all integer points in vector connecting self entity current and previous positions
    // and check if they are in other entity hitbox.
    let path = one.previous_position().as_i16vec2() - one.position().as_i16vec2();
    if path.x != 0 {
        let slope = path.y as f32 / path.x as f32;
        if path.x > 0 {
            for x in 0..=path.x {
                let y = (slope * x as f32).round() as i16;
                for (&point, &one_collider_type) in one.hit_box().iter() {
                    let hit_box_position =
                        (one.position() + point).as_i16vec2() + I16Vec2::new(x, y);
                    let other_position = other.position().as_i16vec2();
                    if hit_box_position.x < other_position.x
                        || hit_box_position.y < other_position.y
                    {
                        continue;
                    }
                    let g_point = (hit_box_position - other_position).as_u16vec2();
                    if let Some(&other_collider_type) = other.hit_box().get(&g_point) {
                        return Some((one_collider_type, other_collider_type));
                    }
                }
            }
        } else {
            for x in path.x..=0 {
                let y = (slope * x as f32).round() as i16;
                for (&point, &one_collider_type) in one.hit_box().iter() {
                    let hit_box_position =
                        (one.position() + point).as_i16vec2() + I16Vec2::new(x, y);
                    let other_position = other.position().as_i16vec2();
                    if hit_box_position.x < other_position.x
                        || hit_box_position.y < other_position.y
                    {
                        continue;
                    }
                    let g_point = (hit_box_position - other_position).as_u16vec2();
                    if let Some(&other_collider_type) = other.hit_box().get(&g_point) {
                        return Some((one_collider_type, other_collider_type));
                    }
                }
            }
        }
    } else {
        if path.y > 0 {
            for y in 0..=path.y {
                let x = path.x;
                for (&point, &one_collider_type) in one.hit_box().iter() {
                    let hit_box_position =
                        (one.position() + point).as_i16vec2() + I16Vec2::new(x, y);
                    let other_position = other.position().as_i16vec2();
                    if hit_box_position.x < other_position.x
                        || hit_box_position.y < other_position.y
                    {
                        continue;
                    }
                    let g_point = (hit_box_position - other_position).as_u16vec2();
                    if let Some(&other_collider_type) = other.hit_box().get(&g_point) {
                        return Some((one_collider_type, other_collider_type));
                    }
                }
            }
        } else {
            for y in path.y..=0 {
                let x = path.x;
                for (&point, &one_collider_type) in one.hit_box().iter() {
                    let hit_box_position =
                        (one.position() + point).as_i16vec2() + I16Vec2::new(x, y);
                    let other_position = other.position().as_i16vec2();
                    if hit_box_position.x < other_position.x
                        || hit_box_position.y < other_position.y
                    {
                        continue;
                    }
                    let g_point = (hit_box_position - other_position).as_u16vec2();
                    if let Some(&other_collider_type) = other.hit_box().get(&g_point) {
                        return Some((one_collider_type, other_collider_type));
                    }
                }
            }
        }
    }

    None
}

fn check_granular_phase_collision<E1, E2>(
    one: &E1,
    other: &E2,
) -> Option<(ColliderType, ColliderType)>
where
    E1: Entity,
    E2: Entity,
{
    for (&point, &one_collider_type) in one.hit_box().iter() {
        let hit_box_position = one.position() + point;
        let other_position = other.position();
        if hit_box_position.x < other_position.x || hit_box_position.y < other_position.y {
            continue;
        }
        let g_point = hit_box_position - other_position;
        if let Some(&other_collider_type) = other.hit_box().get(&g_point) {
            return Some((one_collider_type, other_collider_type));
        }
    }

    None
}

pub fn check_broad_phase_collision<E1, E2>(one: &E1, other: &E2) -> bool
where
    E1: Entity,
    E2: Entity,
{
    // if (one.previous_rect().left() > other.previous_rect().right()
    //     && one.rect().left() > other.rect().right())
    //     || (other.previous_rect().left() > one.previous_rect().right()
    //         && other.rect().left() > one.rect().right())
    //     || (one.previous_rect().bottom() > other.previous_rect().top()
    //         && one.rect().bottom() > other.rect().top())
    //     || (other.previous_rect().bottom() > one.previous_rect().top()
    //         && other.rect().bottom() > one.rect().top())
    // {
    //     return false;
    // }

    let (s1_min, s1_max) = (
        one.previous_position(),
        one.previous_position() + one.size(),
    );
    let (o1_min, o1_max) = (
        other.previous_position(),
        other.previous_position() + other.size(),
    );

    let (s2_min, s2_max) = (one.position(), one.position() + one.size());
    let (o2_min, o2_max) = (other.position(), other.position() + other.size());

    if (s1_min.x > o1_max.x && s2_min.x > o2_max.x)
        || (o1_min.x > s1_max.x && o2_min.x > s2_max.x)
        || (s1_min.y > o1_max.y && s2_min.y > o2_max.y)
        || (o1_min.y > s1_max.y && o2_min.y > s2_max.y)
    {
        return false;
    }

    return true;
}

pub fn are_colliding<E1, E2>(one: &E1, other: &E2) -> Option<(ColliderType, ColliderType)>
where
    E1: Entity,
    E2: Entity,
{
    // Broad phase detection, shortcut if rects cannot intersect
    if !check_broad_phase_collision(one, other) {
        return None;
    }

    // Granular phase detection
    if let Some(colliders) = check_granular_phase_collision(one, other) {
        return Some(colliders);
    }

    // Physical path phase detection
    // This is not perfect, since we don't check if the entities crossed paths while moving,
    // but only one against the other final position. We also don't check if the entity didn't move
    // but rotated somehow. Good enough for us.
    if let Some(colliders) = check_physical_collision(one, other) {
        log::debug!(
            "Found physical collision! {}->{} hit {:#?}",
            one.previous_position(),
            one.position(),
            other.rect()
        );
        return Some(colliders);
    }

    // Do the same swapping entities.
    if let Some(colliders) = check_physical_collision(other, one) {
        log::debug!(
            "Found physical collision! {}->{} hit {:#?}",
            other.previous_position(),
            other.position(),
            one.rect()
        );
        return Some((colliders.1, colliders.0));
    }

    None
}

pub fn inelastic_collision<E1, E2>(one: &mut E1, other: &mut E2, coefficient_of_restituion: f32)
where
    E1: Entity,
    E2: Entity,
{
    one.set_position(one.previous_position());
    other.set_position(other.previous_position());

    let v2_one = if one.mass() == f32::INFINITY {
        one.velocity()
    } else if other.mass() == f32::INFINITY {
        coefficient_of_restituion * (other.velocity() - one.velocity()) + other.velocity()
    } else {
        (coefficient_of_restituion * other.mass() * (other.velocity() - one.velocity())
            + one.mass() * one.velocity()
            + other.mass() * other.velocity())
            / (one.mass() + other.mass())
    };
    let v2_other = if one.mass() == f32::INFINITY {
        coefficient_of_restituion * (other.velocity() - one.velocity()) + one.velocity()
    } else if other.mass() == f32::INFINITY {
        other.velocity()
    } else {
        (coefficient_of_restituion * one.mass() * (other.velocity() - one.velocity())
            + one.mass() * one.velocity()
            + other.mass() * other.velocity())
            / (one.mass() + other.mass())
    };

    // let v2_one = (one.mass()-other.mass())/(one.mass()+other.mass())*one.velocity()+2.0*other.mass()/(one.mass()+other.mass())*other.velocity();
    // let v2_other = 2.0*one.mass()/(one.mass()+other.mass())*one.velocity()+(other.mass()-one.mass())/(one.mass()+other.mass())*other.velocity();

    one.set_velocity(v2_one);
    other.set_velocity(v2_other);
}
