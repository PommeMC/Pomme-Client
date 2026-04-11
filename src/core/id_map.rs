use std::fmt::Debug;

pub trait IdMap<T>: Debug {
    const DEFAULT: i32 = -1;

    fn get_id(&self, thing: &T) -> i32;

    fn by_id(&self, id: i32) -> Option<&T>;

    fn by_id_or_throw(&self, id: i32) -> &T {
        match self.by_id(id) {
            Some(result) => result,
            None => panic!("No value with id {}", id),
        }
    }

    fn get_id_or_throw(&self, value: &T) -> i32
    where
        T: Debug,
    {
        let id = self.get_id(value);
        if id == Self::DEFAULT {
            panic!("Can't find id for '{:?}' in map {:?}", value, self);
        } else {
            id
        }
    }

    fn size(&self) -> usize;
}
