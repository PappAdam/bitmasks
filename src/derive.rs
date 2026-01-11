use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, ItemEnum, Visibility};

pub fn derive(input: &ItemEnum, vis: &Visibility, name: &Ident, bits_type: &Ident) -> TokenStream {
    let bits_struct_name = Ident::new(&format!("{}Bits", name), name.span());
    let variant_idents: Vec<_> = input.variants.iter().map(|v| &v.ident).collect();
    let variant_names: Vec<String> = input.variants.iter().map(|v| v.ident.to_string()).collect();
    let expanded = quote! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #input

    #[repr(transparent)]
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #vis struct #bits_struct_name ( #bits_type );

    impl core::ops::BitOrAssign for #bits_struct_name {
        #[inline]
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }

    impl core::ops::BitAndAssign for #bits_struct_name {
        #[inline]
        fn bitand_assign(&mut self, rhs: Self) {
            self.0 &= rhs.0;
        }
    }

    impl core::ops::BitXorAssign for #bits_struct_name {
        #[inline]
        fn bitxor_assign(&mut self, rhs: Self) {
            self.0 ^= rhs.0;
        }
    }

    impl core::ops::SubAssign for #bits_struct_name {
        #[inline]
        fn sub_assign(&mut self, rhs: Self) {
            self.0 &= !rhs.0;
        }
    }

    impl core::ops::BitOrAssign<#name> for #bits_struct_name {
        #[inline]
        fn bitor_assign(&mut self, rhs: #name) {
            self.0 |= rhs.bits();
        }
    }

    impl core::ops::BitAndAssign<#name> for #bits_struct_name {
        #[inline]
        fn bitand_assign(&mut self, rhs: #name) {
            self.0 &= rhs.bits();
        }
    }

    impl core::ops::BitXorAssign<#name> for #bits_struct_name {
        #[inline]
        fn bitxor_assign(&mut self, rhs: #name) {
            self.0 ^= rhs.bits();
        }
    }

    impl core::ops::SubAssign<#name> for #bits_struct_name {
        #[inline]
        fn sub_assign(&mut self, rhs: #name) {
            self.0 &= !rhs.bits();
        }
    }

    impl core::convert::From<#name> for #bits_struct_name {
        #[inline]
        fn from(val: #name) -> Self {
            Self(val.bits())
        }
    }

    impl core::convert::From<#bits_struct_name> for #bits_type {
        #[inline]
        fn from(val: #bits_struct_name) -> Self {
            val.0
        }
    }

    impl core::convert::From<#bits_type> for #bits_struct_name {
        #[inline]
        fn from(val: #bits_type) -> Self {
            Self(val)
        }
    }

    impl core::convert::From<#name> for #bits_type {
        #[inline]
        fn from(val: #name) -> Self {
            val.bits()
        }
    }


    impl #name {
        #[inline]
        pub fn bits(&self) -> #bits_type {
            *self as #bits_type
        }
    }


    impl core::ops::BitOr for #name {
        type Output = #bits_struct_name;
        #[inline]
        fn bitor(self, rhs: Self) -> Self::Output {
            #bits_struct_name(self.bits() | rhs.bits())
        }
    }

    impl core::ops::BitAnd for #name {
        type Output = #bits_struct_name;
        #[inline]
        fn bitand(self, rhs: Self) -> Self::Output {
            #bits_struct_name(self.bits() & rhs.bits())
        }
    }

    impl core::ops::BitXor for #name {
        type Output = #bits_struct_name;
        #[inline]
        fn bitxor(self, rhs: Self) -> Self::Output {
            #bits_struct_name(self.bits() ^ rhs.bits())
        }
    }

    impl core::ops::Not for #name {
        type Output = #bits_struct_name;
        #[inline]
        fn not(self) -> Self::Output {
            #bits_struct_name(!self.bits())
        }
    }

    impl core::ops::BitOr for #bits_struct_name {
        type Output = Self;
        #[inline]
        fn bitor(self, rhs: Self) -> Self {
            Self(self.0 | rhs.0)
        }
    }

    impl core::ops::BitAnd for #bits_struct_name {
        type Output = Self;
        #[inline]
        fn bitand(self, rhs: Self) -> Self {
            Self(self.0 & rhs.0)
        }
    }

    impl core::ops::BitXor for #bits_struct_name {
        type Output = Self;
        #[inline]
        fn bitxor(self, rhs: Self) -> Self {
            Self(self.0 ^ rhs.0)
        }
    }

    impl core::ops::Not for #bits_struct_name {
        type Output = Self;
        #[inline]
        fn not(self) -> Self {
            Self(!self.0)
        }
    }

    impl core::cmp::PartialEq<#name> for #bits_struct_name {
        #[inline]
        fn eq(&self, other: &#name) -> bool {
            self.0 == other.bits()
        }
    }

    impl core::cmp::PartialEq<#bits_struct_name> for #name {
        #[inline]
        fn eq(&self, other: &#bits_struct_name) -> bool {
            self.bits() == other.0
        }
    }

    impl core::fmt::Debug for #bits_struct_name {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let raw_value = self.0;
            let mut first = true;

            f.write_str(concat!(stringify!(#bits_struct_name), "("))?;

            #(
                {
                    let mask_val = #name::#variant_idents as #bits_type;

                    if (raw_value & mask_val) == mask_val && mask_val != (0 as #bits_type) {
                        if !first {
                            f.write_str(" | ")?;
                        }

                        f.write_str(#variant_names)?;

                        first = false;
                    }
                }
            )*

            if first {
                if raw_value == (0 as #bits_type) {
                    f.write_str("0x0")?;
                } else {
                    // Print unknown bits in Hex
                    core::fmt::LowerHex::fmt(&raw_value, f)?;
                }
            }

            f.write_str(")")
        }
    }
    };

    TokenStream::from(expanded)
}
