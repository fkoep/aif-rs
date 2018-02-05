use futures::sync::oneshot;
use futures::prelude::*;
use std::error::Error;

#[derive(Debug)]
pub enum ApcError {
    /// Fault lies somewhere within the currently running program.
    Local(Box<Error + Send + Sync>),
    /// Fault lies with a third party/the environment interfering somehow.
    Connection(Box<Error + Send + Sync>),
    /// Fault lies with the remote endpoint(s).
    Remote(Box<Error + Send + Sync>),
}

pub type ApcResult<T> = Result<T, ApcError>;

// TODO what abstracts the sending end of a future?
pub struct Complete<T> {
    // FIXME(fnbox) use FnBox or Box<FnOnce> instead
    f: Box<FnMut(T) + Send + Sync>   
}

impl<T> Complete<T> {
    pub fn new<F>(f: F) -> Self
        where F: FnOnce(T) + Send + Sync + 'static
    {
        let mut opt = Some(f);
        Self{ f: Box::new(move |r| (opt.take().unwrap())(r)) }
    }
    pub fn complete(mut self, r: T){
        (self.f)(r);
    }
}

// TODO Remove? Naming?
pub type ApcFuture<Ret> = Box<Future<Item = Ret, Error = ApcError> + Send>;

pub trait Interface: Send + Sync + 'static {
    type Call: Send + 'static;
    type Return: Send + 'static;

    // TODO awfully long type-signature, can we cut this down?
    fn send_call(&self, call: Self::Call, res: Complete<ApcResult<Self::Return>>) -> StartSend<(), ApcError>;

    fn start_call(&self, call: Self::Call) -> ApcFuture<Self::Return> {
        let (ret_tx, ret_rx) = oneshot::channel::<ApcResult<Self::Return>>();
        let _ = self.send_call(call, Complete::new(|ret| { let _ = ret_tx.send(ret); }));
        Box::new(ret_rx
             .map_err(|e| ApcError::Local(Box::new(e)))
             .and_then(|res| res))
    }
}
