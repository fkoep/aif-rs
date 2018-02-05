use apc::*;
use futures::sync::mpsc;
use futures::prelude::*;

pub type UnboundedSender<I>
    where I: Interface + ?Sized
= mpsc::UnboundedSender<(I::Call, Complete<ApcResult<I::Return>>)>;

impl<Call, Ret> Interface for mpsc::UnboundedSender<(Call, Complete<ApcResult<Ret>>)>
    where Call: Send + 'static, Ret: Send + 'static
{
    type Call = Call;
    type Return = Ret;

    fn send_call(&self, call: Self::Call, res: Complete<ApcResult<Self::Return>>) -> StartSend<(), ApcError> {
        self.unbounded_send((call, res))
            .map(|_| AsyncSink::Ready)
            .map_err(|e| ApcError::Local(e.to_string().into())) // TODO error?
    }
}

pub type UnboundedReceiver<I>
    where I: Interface + ?Sized
= mpsc::UnboundedReceiver<(I::Call, Complete<ApcResult<I::Return>>)>;

pub fn unbounded<I>() -> (UnboundedSender<I>, UnboundedReceiver<I>)
    where I: Interface + ?Sized
{
    mpsc::unbounded()
}
