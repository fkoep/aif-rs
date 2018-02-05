#[doc(hidden)]
pub extern crate futures;

mod apc;
mod unbounded;

pub use apc::{Complete, Interface};
pub use apc::ApcError as Error;
pub use apc::ApcResult as Result;
pub use apc::ApcFuture as Future;
pub use unbounded::*;

#[macro_export]
macro_rules! apc_interfaces {
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
                    $args : $args_ty
                ),*}
            ),*}

            impl Call {
                pub fn apply(self, __backend: &mut Backend) -> Return {
                    match self {$(
                        Call::$method{ $($args),* } => Return::$method(__backend.$method($($args,)*))
                    ),*}
                }
            }

            $($(#[$return_attrs])*)*
            $($(#[$data_attrs])*)*
            #[allow(non_camel_case_types)]
            pub enum Return {$(
                $($(#[$m_return_attrs])*)*
                $($(#[$m_data_attrs])*)*
                $method(($($ret_ty)*))
            ),*}

            $(#[$attrs])*
            pub trait $intf: $crate::Interface<Call = Call, Return = Return> {$(
                $(#[$m_attrs])*
                #[allow(unreachable_patterns)]
                fn $method(&self, $($args : $args_ty),*) -> $crate::ApcFuture<($($ret_ty)*)> {
                    use $crate::futures::Future;
                    Box::new($crate::Interface::start_call(self, Call::$method{$($args),*})
                        .map(|ret| match ret { Return::$method(r) => r, _ => unreachable!() }))
                }
            )*}

            impl<__T> $intf for __T
                where __T: $crate::Interface<Call = Call, Return = Return> + ?Sized
            {}

            $($(#[$backend_attrs])*)*
            pub trait Backend {
                $(
                    $($(#[$m_backend_attrs])*)*
                    fn $method(&mut self, $($args : $args_ty,)*) $(-> $ret_ty)*;
                )*
            }

            pub fn serve<S>(s: &mut S, backend: &mut Backend) -> $crate::futures::Poll<Option<()>, S::Error> 
                where S: $crate::futures::Stream<Item = (Call, $crate::Complete<Return>)>
            {
                use $crate::futures::Stream;
                s.map(|(call, ret_tx)| { let _ = ret_tx.complete(call.apply(backend)); }).poll()
            }
        }
        //FIXME(rustc, look for issue...) remove  workaround
        //pub use self::$mod::$intf;
        pub type $intf = self::$mod::$intf<Call = self::$mod::Call, Return = self::$mod::Return>;
    )*}
}
