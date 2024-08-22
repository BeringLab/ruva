// To Support Bulk Insert Operation
#[macro_export]
macro_rules! prepare_bulk_operation {
    (
        $subject:expr, $($field:ident:$field_type:ty),*
    ) => {
        $(
            let mut $field:Vec<$field_type> = Vec::with_capacity($subject.len());
        )*

        $subject.iter().for_each(|subj|{
            $(
                $field.push(subj.$field.clone());
            )*
        }
        )

    };
    (
        $subject:expr, $($field:ident():$field_type:ty),*
    ) =>{
        $(
            let mut $field:Vec<$field_type> = Vec::with_capacity($subject.len());
        )*

        $subject.iter().for_each(|subj|{
            $(
                $field.push(subj.$field().to_owned());
            )*
        }
        )
    }
}

#[macro_export]
macro_rules! make_smart_pointer {
    ($name:ident $(<$($gens:ident),*>)?, $inner:ty) => {
        impl$(<$($gens),*>)? std::ops::Deref for $name$(<$($gens),*>)? {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl$(<$($gens),*>)? std::ops::DerefMut for $name$(<$($gens),*>)? {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };

    ($name:ident $(<$($gens:ident),*>)?, $inner:ty, $identifier:ident)=>{
        impl$(<$($gens),*>)? std::ops::Deref for $name$(<$($gens),*>)? {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                &self.$identifier
            }
        }
        impl$(<$($gens),*>)? std::ops::DerefMut for $name$(<$($gens),*>)? {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$identifier
            }
        }
    }

}

#[macro_export]
macro_rules! make_conversion {
	($type_name:ident, $($target:ty),*) => {
        $(
            impl ::core::convert::From<$target> for $type_name {
                fn from(value: $target) -> $type_name {
                    $type_name(value.into())
                }
            }
        )*
    };
	($type_name:ident<$target:ty>) => {
		impl ::core::convert::From<$target> for $type_name<$target> {
			fn from(value: $target) -> $type_name<$target> {
				$type_name(value.into())
			}
		}
    };
}

#[macro_export]
macro_rules! error {

		(

		) => {
			|err| {
                ::ruva::tracing::error!("{:?} {}:{}", err, file!(),line!()); err
            }
		};
        (
            $stmt:expr

            $(, $arg:expr)* $(,)?

        ) => {
            ::ruva::tracing::error!("{} {}:{}", format!($stmt, $($arg),*),file!(),line!())
        };
	}

#[allow(unused)]
macro_rules! all_the_tuples {
	($name:ident) => {
		$name!([]);
		$name!([T1]);
		$name!([T1, T2]);
		$name!([T1, T2, T3]);
		$name!([T1, T2, T3, T4]);
		$name!([T1, T2, T3, T4, T5]);
		$name!([T1, T2, T3, T4, T5, T6]);
		$name!([T1, T2, T3, T4, T5, T6, T7]);
		$name!([T1, T2, T3, T4, T5, T6, T7, T8]);
		$name!([T1, T2, T3, T4, T5, T6, T7, T8, T9]);
		$name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10]);
	};
}
