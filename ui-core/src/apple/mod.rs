/// Generate Apple platform `Prop` statics from a compact property list.
#[macro_export]
macro_rules! apple_view_props {
    ($component:ident on $view:ident { $vis:vis $name:ident : String ; $($rest:tt)* }) => {
        ::paste::paste! {
            $vis static [<PROP_ $name:upper>]: $crate::Prop<$component, objc2::rc::Retained<$view>, String> =
                $crate::Prop::new(|view, value| {
                    view.[<set $name:camel>](
                        &::objc2_foundation::NSString::from_str(&value)
                    );
                });
        }
        $crate::apple_view_props!($component on $view { $($rest)* });
    };
    ($component:ident on $view:ident { $vis:vis $name:ident : $ty:ty ; $($rest:tt)* }) => {
        ::paste::paste! {
            $vis static [<PROP_ $name:upper>]: $crate::Prop<$component, objc2::rc::Retained<$view>, $ty> =
                $crate::Prop::new(|view, value| {
                    view.[<set $name:camel>](value);
                });
        }
        $crate::apple_view_props!($component on $view { $($rest)* });
    };
    ($component:ident on $view:ident { }) => {};
}

pub(crate) mod action_target;
