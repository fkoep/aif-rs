#![feature(macro_vis_matcher)]

pub extern crate futures;

pub trait Facade: Send + Sync + 'static {
    // TODO?
    // type Call
    // fn send_call(
    // fn gateway_closed(&self) -> bool;
}

/// TODO Error-type?
pub type Future<T> =
    Box<futures::Future<Item = T, Error = futures::sync::oneshot::Canceled> + Send>;
pub type OneshotTx<T> = futures::sync::oneshot::Sender<T>;
pub type OneshotRx<T> = futures::sync::oneshot::Receiver<T>;

// #[doc(hidden)]
#[macro_export]
macro_rules! _api_facades {
    (@invoke_each [$($macs:ident),*] $body:tt) => {
        $($macs! $body)*
    };

    (/* @toplevel */ [] []) => {};

    (/* @toplevel */ [$($attrs:meta),*] [$($gen_macs:ident),*]
        #[gen($($next_gen_macs:ident),+ $(,)*)] 
        $($rest:tt)+
    ) => {
        _api_facades!{ /* @toplevel */ [$($attrs),*] [$($gen_macs,)* $($next_gen_macs),+]
            $($rest)+
        }
    };

    (/* @toplevel */ [$($attrs:meta),*] [$($gen_macs:ident),*]
        #[$next_attr:meta] 
        $($rest:tt)+
    ) => {
        _api_facades!{ /* @toplevel */ [$($attrs,)* $next_attr] [$($gen_macs),*]
            $($rest)+
        }
    };

    (/* @toplevel */ [$($attrs:meta),*] [$($gen_macs:ident),*]
        $vis:vis facade $module:ident :: $facade:ident $(<$($ty_params:ident),+ $(,)*>)* 
            $(where ($($ty_bounds:tt)+))*
        { 
            $(
                $(#[$method_attrs:meta])* 
                fn $methods:ident($($args:ident: $arg_tys:ty),* $(,)*) $(-> $ret_tys:ty)*;
            )*
        } 
        $($rest:tt)*
    ) => {
        #[allow(unused)]
        $vis mod $module {
            use super::*;

            $(#[$attrs])*
            pub trait $facade $(<$($ty_params),+>)*: ::api_facades::Facade
                where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
            {
                $(
                    $(#[$method_attrs])*
                    fn $methods(&self, $($args: $arg_tys),*) -> ::api_facades::OneshotRx<($($ret_tys)*)>;
                )*
            }

            _api_facades!(@invoke_each [$($gen_macs),*] {
                $module $facade [$([$($ty_params),+])*] [$([$($ty_bounds)+])*]
                    $(
                        $(#[$method_attrs])* 
                        fn $methods($($args: $arg_tys),*) -> ($($ret_tys)*);
                    )*
            });
        }
        $vis use $module::$facade;

        _api_facades!{ /* @toplevel */ [] []
            $($rest)*
        }
    };
}

#[macro_export]
macro_rules! api_facades {
    ($($tt:tt)*) => { 
        _api_facades!{
            /* @toplevel */ [] []
            $($tt)*
        }
    };
}

#[macro_export]
macro_rules! task_facade {
    ($module:ident $facade:ident [$([$($ty_params:ident),+])*] [$([$($ty_bounds:tt)+])*]
        $(
            $(#[$method_attrs:meta])* fn $methods:ident($($args:ident: $arg_tys:ty),*) -> $ret_tys:ty;
        )*
    ) => {
        pub trait TaskBackendAsync $(<$($ty_params),+>)*
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            $(fn $methods(&mut self, $($args: $arg_tys,)* __out: ::api_facades::OneshotTx<$ret_tys>);)*
        }

        pub trait TaskBackend $(<$($ty_params),+>)*
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            $(fn $methods(&mut self, $($args: $arg_tys,)*) -> $ret_tys;)*
        }

        impl<__T, $($($ty_params),+)*> TaskBackendAsync $(<$($ty_params),+>)* for __T
            where __T: TaskBackend $(<$($ty_params),+>)*, $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            $(
                fn $methods(&mut self, $($args: $arg_tys,)* __out: ::api_facades::OneshotTx<$ret_tys>){
                    let _ = __out.send(self.$methods($($args)*));
                }
            )*
        }

        #[allow(non_camel_case_types)]
        enum TaskCall $(<$($ty_params),+>)* 
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            $(
                $methods{ 
                    $($args: $arg_tys,)*
                    __out: ::api_facades::OneshotTx<$ret_tys>,
                },
            )*
        }

        impl $(<$($ty_params),+>)* TaskCall $(<$($ty_params),+>)*
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            fn apply<__B>(self, __backend: &mut __B)
                where __B: TaskBackendAsync $(<$($ty_params),+>)* + ?Sized
            {
                match self {
                    $(TaskCall::$methods{ $($args,)* __out } => __backend.$methods($($args,)* __out),)*
                }
            }
        }

        pub struct TaskFacade $(<$($ty_params),+>)* 
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            tx: ::api_facades::futures::sync::mpsc::UnboundedSender<TaskCall $(<$($ty_params),+>)* >
        }

        impl $(<$($ty_params),+>)* Clone for TaskFacade $(<$($ty_params),+>)*
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            fn clone(&self) -> Self { Self{ tx: self.tx.clone() } }
        }

        impl $(<$($ty_params),+>)* ::api_facades::Facade for TaskFacade $(<$($ty_params),+>)*
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {}

        impl $(<$($ty_params),+>)* $facade $(<$($ty_params),+>)* for TaskFacade $(<$($ty_params),+>)*
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            $(
                fn $methods(&self, $($args: $arg_tys,)*) -> ::api_facades::OneshotRx<$ret_tys> {
                    use ::api_facades::futures::sync::oneshot;

                    let (__tx, __rx) = oneshot::channel();
                    // TODO what do with errors?
                    let _ = self.tx.unbounded_send(TaskCall::$methods{ $($args,)* __out: __tx });
                    __rx
                }
            )*
        }

        pub struct TaskServer $(<$($ty_params),+>)* 
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            rx: ::api_facades::futures::sync::mpsc::UnboundedReceiver<TaskCall $(<$($ty_params),+>)* >
        }

        impl $(<$($ty_params),+>)* TaskServer $(<$($ty_params),+>)*
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            pub fn serve<__B>(&mut self, backend: &mut __B) -> bool
                where __B: TaskBackendAsync $(<$($ty_params),+>)* + ?Sized
            {
                use ::api_facades::futures::{Async, Stream};

                match self.rx.poll() {
                    Ok(Async::Ready(Some(call))) => { call.apply(backend); true }
                    Ok(Async::Ready(None)) => false,
                    Ok(Async::NotReady) => false,
                    _ => unimplemented!() // TODO
                }
            }
        }
        
        pub fn task_channel $(<$($ty_params),+>)* () -> (TaskFacade $(<$($ty_params),+>)*, TaskServer $(<$($ty_params),+>)*) 
            where $($($ty_params: Send + 'static,)+)* $($($ty_bounds)+)*
        {
            use ::api_facades::futures::sync::mpsc;

            let (tx, rx) = mpsc::unbounded();
            (TaskFacade{ tx }, TaskServer{ rx })
        }
    }
}
