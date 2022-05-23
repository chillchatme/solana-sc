use crate::StakingErrorCode;
use anchor_lang::prelude::*;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

pub trait GetLazyVector<'a, T> {
    fn get_vector(&self) -> Result<LazyVector<'a, T>>;
}

pub struct LazyVector<'a, T> {
    offset: usize,
    size: usize,
    elem_size: usize,
    data: Rc<RefCell<&'a mut [u8]>>,
    marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T> LazyVector<'a, T>
where
    T: AnchorDeserialize + AnchorSerialize,
{
    pub fn new(
        offset: usize,
        size: usize,
        elem_size: usize,
        data: Rc<RefCell<&'a mut [u8]>>,
    ) -> Result<Self> {
        let data_len = data.borrow().len();

        let free_space = data_len
            .checked_sub(offset)
            .and_then(|v| v.checked_div(elem_size))
            .unwrap();

        require_gte!(free_space, size, StakingErrorCode::WrongVectorSize);

        Ok(LazyVector {
            offset,
            size,
            elem_size,
            data,
            marker: PhantomData,
        })
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn get(&self, index: usize) -> Result<T> {
        require_gt!(self.size, index, StakingErrorCode::OutOfBounds);
        let from = index
            .checked_mul(self.elem_size)
            .and_then(|v| v.checked_add(self.offset))
            .unwrap();

        let to = from.checked_add(self.elem_size).unwrap();
        let data = self.data.borrow();
        let mut slice = &data[from..to];

        T::deserialize(&mut slice).map_err(Into::into)
    }

    pub fn set(&mut self, index: usize, value: &T) -> Result<()> {
        require_gt!(self.size, index, StakingErrorCode::OutOfBounds);

        let from = index
            .checked_mul(self.elem_size)
            .and_then(|v| v.checked_add(self.offset))
            .unwrap();

        let to = from.checked_add(self.elem_size).unwrap();
        let mut data = self.data.borrow_mut();

        let raw_value = value.try_to_vec()?;
        require_eq!(raw_value.len(), self.elem_size);

        data[from..to].copy_from_slice(&raw_value);

        Ok(())
    }

    pub fn clear(&mut self) {
        let from = self.offset;
        let to = self
            .size
            .checked_mul(self.elem_size)
            .and_then(|v| v.checked_add(from))
            .unwrap();

        self.data.borrow_mut()[from..to].fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};
    use std::mem;

    macro_rules! test_lazy_vector {
        ($x:tt, $func_name:ident) => {
            #[test]
            fn $func_name() {
                const ELEM_SIZE: usize = mem::size_of::<$x>();
                const VECTOR_SIZE: usize = 256;

                let mut rng = thread_rng();

                let vec = (0..VECTOR_SIZE).map(|_| rng.gen()).collect::<Vec<$x>>();

                let mut buffer = [0; VECTOR_SIZE * ELEM_SIZE];
                let data = Rc::new(RefCell::new(buffer.as_mut()));

                assert!(LazyVector::<$x>::new(1, VECTOR_SIZE, ELEM_SIZE, data.clone()).is_err());
                assert!(
                    LazyVector::<$x>::new(0, VECTOR_SIZE + 1, ELEM_SIZE, data.clone()).is_err()
                );

                let mut lazy_vector = LazyVector::new(0, VECTOR_SIZE, ELEM_SIZE, data).unwrap();
                for (index, item) in vec.iter().enumerate() {
                    lazy_vector.set(index, item).unwrap();
                }

                for (index, item) in vec.iter().enumerate() {
                    assert_eq!(*item, lazy_vector.get(index).unwrap());
                }

                for (index, item) in vec.iter().enumerate() {
                    let from = index * ELEM_SIZE;
                    let to = from + ELEM_SIZE;
                    let value = $x::deserialize(&mut &buffer[from..to]).unwrap();
                    assert_eq!(*item, value)
                }
            }
        };
    }

    test_lazy_vector!(bool, lazy_vector_bool);
    test_lazy_vector!(u8, lazy_vector_u8);
    test_lazy_vector!(u16, lazy_vector_u16);
    test_lazy_vector!(u32, lazy_vector_u32);
    test_lazy_vector!(u64, lazy_vector_u64);
    test_lazy_vector!(u128, lazy_vector_u128);

    #[test]
    fn lazy_vector_with_shift() {
        let mut rng = thread_rng();
        let mut buffer = [0; 1024];
        let from = 924;

        // 1024 - 924 = 100
        // 100 / 8 = 12
        let vec = (0..12)
            .map(|_| rng.gen_range(0..u64::MAX))
            .collect::<Vec<_>>();

        let data = Rc::new(RefCell::new(buffer.as_mut()));

        assert!(LazyVector::<u64>::new(from, 13, 8, data.clone()).is_err());
        let mut lazy_vector = LazyVector::new(from, 12, 8, data).unwrap();

        for (index, item) in vec.iter().enumerate() {
            lazy_vector.set(index, item).unwrap();
        }

        for (index, item) in vec.iter().enumerate() {
            assert_eq!(*item, lazy_vector.get(index).unwrap())
        }

        for (index, item) in vec.iter().enumerate() {
            let value = u64::from_le_bytes(
                buffer[from + 8 * index..from + 8 * (index + 1)]
                    .try_into()
                    .unwrap(),
            );

            assert_eq!(*item, value)
        }
    }

    #[test]
    fn clear() {
        let mut vector = vec![0, 1, 2, 3, 4, 5];
        let buffer: &mut [u8] = &mut vector;
        let data = Rc::new(RefCell::new(buffer));

        let mut lazy_vector = LazyVector::<u8>::new(2, 3, 1, data).unwrap();
        assert_eq!(lazy_vector.get(0).unwrap(), 2);
        assert_eq!(lazy_vector.get(1).unwrap(), 3);
        assert_eq!(lazy_vector.get(2).unwrap(), 4);
        assert!(lazy_vector.get(3).is_err());

        lazy_vector.clear();
        assert_eq!(lazy_vector.get(0).unwrap(), 0);
        assert_eq!(lazy_vector.get(1).unwrap(), 0);
        assert_eq!(lazy_vector.get(2).unwrap(), 0);
        assert!(lazy_vector.get(3).is_err());

        assert_eq!(vector[0], 0);
        assert_eq!(vector[1], 1);
        assert_eq!(vector[2], 0);
        assert_eq!(vector[3], 0);
        assert_eq!(vector[4], 0);
        assert_eq!(vector[5], 5);
    }
}
