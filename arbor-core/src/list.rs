use crate::{Node, Status};

#[allow(async_fn_in_trait)]
pub trait NodeList<Ctx> {
    const LEN: usize;

    async fn tick_at(&mut self, index: usize, ctx: &mut Ctx) -> Status;
    fn reset_range(&mut self, start: usize);
    fn reset_all(&mut self);
}

impl<Ctx, T, const N: usize> NodeList<Ctx> for [T; N]
where
    T: Node<Ctx>,
{
    const LEN: usize = N;

    async fn tick_at(&mut self, index: usize, ctx: &mut Ctx) -> Status {
        if index >= N {
            panic!("child index out of bounds: {index} >= {N}");
        }

        self[index].tick(ctx).await
    }

    fn reset_range(&mut self, start: usize) {
        for child in self.iter_mut().skip(start) {
            child.reset();
        }
    }

    fn reset_all(&mut self) {
        for child in self.iter_mut() {
            child.reset();
        }
    }
}

macro_rules! impl_node_list_for_tuple {
    ($len:expr, $( $idx:tt => $ty:ident ),+ $(,)?) => {
        impl<Ctx, $( $ty ),+> NodeList<Ctx> for ($( $ty, )+)
        where
            $( $ty: Node<Ctx>, )+
        {
            const LEN: usize = $len;

            async fn tick_at(&mut self, index: usize, ctx: &mut Ctx) -> Status {
                match index {
                    $( $idx => self.$idx.tick(ctx).await, )+
                    _ => panic!(
                        "child index out of bounds: {index} >= {}",
                        Self::LEN
                    ),
                }
            }

            fn reset_range(&mut self, start: usize) {
                $(
                    if start <= $idx {
                        self.$idx.reset();
                    }
                )+
            }

            fn reset_all(&mut self) {
                $( self.$idx.reset(); )+
            }
        }
    };
}

impl_node_list_for_tuple!(1, 0 => A);
impl_node_list_for_tuple!(2, 0 => A, 1 => B);
impl_node_list_for_tuple!(3, 0 => A, 1 => B, 2 => C);
impl_node_list_for_tuple!(4, 0 => A, 1 => B, 2 => C, 3 => D);
impl_node_list_for_tuple!(5, 0 => A, 1 => B, 2 => C, 3 => D, 4 => E);
impl_node_list_for_tuple!(6, 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F);
impl_node_list_for_tuple!(7, 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G);
impl_node_list_for_tuple!(8, 0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H);
impl_node_list_for_tuple!(
    9,
    0 => A,
    1 => B,
    2 => C,
    3 => D,
    4 => E,
    5 => F,
    6 => G,
    7 => H,
    8 => I
);
impl_node_list_for_tuple!(
    10,
    0 => A,
    1 => B,
    2 => C,
    3 => D,
    4 => E,
    5 => F,
    6 => G,
    7 => H,
    8 => I,
    9 => J
);
impl_node_list_for_tuple!(
    11,
    0 => A,
    1 => B,
    2 => C,
    3 => D,
    4 => E,
    5 => F,
    6 => G,
    7 => H,
    8 => I,
    9 => J,
    10 => K
);
impl_node_list_for_tuple!(
    12,
    0 => A,
    1 => B,
    2 => C,
    3 => D,
    4 => E,
    5 => F,
    6 => G,
    7 => H,
    8 => I,
    9 => J,
    10 => K,
    11 => L
);
