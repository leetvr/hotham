use legion::Entity;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parent(pub Entity);

#[cfg(test)]
mod tests {
    use super::*;
    use legion::{world::Duplicate, EntityStore, IntoQuery, World};

    #[test]
    pub fn test_parent_clone() {
        let mut world = World::default();
        let e1 = world.push((1 as usize,));
        let e2 = world.push((2, Parent(e1), e1));
        let e3 = e2.clone();

        {
            let e3_entity = world.entry(e3).unwrap();
            let e3_parent = e3_entity.get_component::<Parent>().unwrap().0;
            let mut e1 = world.entry(e3_parent).unwrap();
            assert_eq!(e1.get_component::<usize>().unwrap(), &1);
            *(e1.get_component_mut::<usize>().unwrap()) = 3;
        }

        let mut clone_world = World::default();
        let mut merger = Duplicate::default();
        merger.register_clone::<Parent>();
        merger.register_clone::<Entity>();
        merger.register_clone::<usize>();

        clone_world.clone_from(&world, &legion::any(), &mut merger);

        let mut query = <&Entity>::query();
        unsafe {
            query.for_each_unchecked(&clone_world, |e| {
                let parent_entity = clone_world.entry_ref(*e).unwrap();
                assert_eq!(parent_entity.get_component::<usize>().unwrap(), &3);
            });
        }

        let mut query = <&Parent>::query();
        unsafe {
            query.for_each_unchecked(&clone_world, |p| {
                let parent_entity = clone_world.entry_ref(p.0).unwrap();
                assert_eq!(parent_entity.get_component::<usize>().unwrap(), &3);
            });
        }
    }
}
