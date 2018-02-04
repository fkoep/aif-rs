#[doc(hidden)]
pub extern crate futures;

use futures::prelude::*;
use futures::sync::{mpsc, oneshot};
use std::error::Error;

#[derive(Debug)]
pub enum CallError {
    /// Fault lies somewhere within the currently running program.
    Local(Box<Error + Send + Sync>),
    /// Fault lies with a third party/the environment interfering somehow.
    Connection(Box<Error + Send + Sync>),
    /// Fault lies with the remote endpoint.
    Remote(Box<Error + Send + Sync>),
}

// TODO 
pub type OneshotTx<R> = oneshot::Sender<R>;
// TODO Naming? 
pub type OneshotRx<R> = Box<Future<Item = R, Error = CallError> + Send>;

/// TODO remove this completely?
pub trait Interface<Call, Ret>: Send + Sync + 'static
    where Call: Send + 'static, Ret: Send + 'static
{
    // FIXME
    // type Return: Send + 'static;

    fn forward_call(&self, call: Call, ret: OneshotTx<Ret>);

    fn start_call(&self, call: Call) -> OneshotRx<Ret> {
        let (ret_tx, ret_rx) = oneshot::channel();
        self.forward_call(call, ret_tx);
        Box::new(ret_rx.map_err(|e| CallError::Local(Box::new(e))))
    }
}

// TODO?
// pub trait Backend<Call, Ret> {
//     fn complete_call(&mut self, call: Call) -> Ret;
// }

// ++++++++++++++++++++ unbounded ++++++++++++++++++++

pub struct UnboundedSender<Call, Ret> {
    tx: mpsc::UnboundedSender<(Call, OneshotTx<Ret>)>,
}

impl<Call, Ret> Clone for UnboundedSender<Call, Ret> 
    where Call: Send + 'static, Ret: Send + 'static
{
    fn clone(&self) -> Self {
        Self{ tx: self.tx.clone() }
    }
}

impl<Call, Ret> Interface<Call, Ret> for UnboundedSender<Call, Ret> 
    where Call: Send + 'static, Ret: Send + 'static
{
    fn forward_call(&self, call: Call, ret: OneshotTx<Ret>){
        let _ = self.tx.unbounded_send((call, ret)); // TODO? handle err
    }
}

// TODO
pub type UnboundedReceiver<Call, Ret> = mpsc::UnboundedReceiver<(Call, OneshotTx<Ret>)>;

pub fn unbounded<Call, Ret>() -> (UnboundedSender<Call, Ret>, UnboundedReceiver<Call, Ret>) 
    where Call: Send + 'static, Ret: Send + 'static
{
    let (tx, rx) = mpsc::unbounded();
    (UnboundedSender{ tx }, rx)
}

#[macro_export]
macro_rules! api_bridges {
    // TODO remove parenthesis-workaround in `where ()`
    ($(
        $(#[$attrs:meta])*
        $(@[mod_attrs($($mod_attrs:meta),* $(,)*)])*
        $(@[call_attrs($($call_attrs:meta),* $(,)*)])*
        $(@[return_attrs($($return_attrs:meta),* $(,)*)])*
        $(@[data_attrs($($data_attrs:meta),* $(,)*)])*
        $(@[backend_attrs($($backend_attrs:meta),* $(,)*)])*
        intf $mod:ident :: $intf:ident $(<$($ty_params:ident),+>)*
            $(where ($($ty_bounds:tt)+))*
        {$(
            $(#[$m_attrs:meta])*
            $(@[call_attrs($($m_call_attrs:meta),* $(,)*)])*
            $(@[return_attrs($($m_return_attrs:meta),* $(,)*)])*
            $(@[data_attrs($($m_data_attrs:meta),* $(,)*)])*
            $(@[backend_attrs($($m_backend_attrs:meta),* $(,)*)])*
            fn $method:ident($($args:ident : $args_ty:ty),* $(,)*) $(-> $ret_ty:ty)*;
        )*}
    )*) => {$(
        $($(#[$mod_attrs])*)*
        pub mod $mod {
            $($(#[$call_attrs])*)*
            $($(#[$data_attrs])*)*
            #[allow(non_camel_case_types)]
            pub enum Call {$(
                $($(#[$m_call_attrs])*)*
                $($(#[$m_data_attrs])*)*
                $method{$(
                    $args : $arg_ty
                ),*}
            ),*}

            $($(#[$return_attrs])*)*
            $($(#[$data_attrs])*)*
            #[allow(non_camel_case_types)]
            pub enum Return {$(
                $($(#[$m_return_attrs])*)*
                $($(#[$m_data_attrs])*)*
                $method(($($ret_ty)*))
            ),*}

            $(#[$attrs])*
            pub trait $intf: $crate::Interface<Call, Return> {$(
                $(#[$m_attrs])*
                #[allow(unreachable_patterns)]
                fn $method(&self, $($args : $arg_ty),*) -> $crate::OneshotRx<($($ret_ty)*)> {
                    use $crate::futures::Future;
                    Box::new($crate::Interface::start_call(self, Call::$method{$($args),*})
                        .map(|ret| match ret { Return::$method(r) => r, _ => unreachable!() }))
                }
            )*}

            impl<__T> $intf for __T
                where __T: $crate::Interface<Call, Return> + ?Sized
            {}

            $($(#[$backend_attrs])*)*
            pub trait Backend {
                $(
                    $($(#[$m_backend_attrs])*)*
                    fn $method(&mut self, $($args : $arg_ty,)*) $(-> $ret_ty)*;
                )*
            
            }

            // TODO move this to api_bridges?
            pub fn complete_call(__backend: &mut Backend, call: Call) -> Return {
                match call {$(
                    Call::$method{ $($args),* } => Return::$method(__backend.$method($($args,)*))
                ),*}
            }

            // TODO move this to api_bridges?
            pub fn serve_call<S>(s: &mut S, backend: &mut Backend) -> $crate::futures::Poll<Option<()>, S::Error> 
                where S: $crate::futures::Stream<Item = (Call, $crate::OneshotTx<Return>)>
            {
                use $crate::futures::Stream;
                s.map(|(call, ret_tx)| { let _ = ret_tx.send(complete_call(backend, call)); }).poll()
            }
        }
        pub use $mod::$intf;
    )*}
}
