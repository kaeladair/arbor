use crate::Status;

#[allow(async_fn_in_trait)]
pub trait Node<Ctx> {
    async fn tick(&mut self, ctx: &mut Ctx) -> Status;

    fn reset(&mut self) {}
}
