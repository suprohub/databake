// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use super::*;
extern crate alloc;

impl<T> Bake for alloc::borrow::Cow<'_, T>
where
    T: ?Sized + ToOwned,
    for<'a> &'a T: Bake,
{
    fn bake(&self, ctx: &CrateEnv) -> TokenStream {
        ctx.insert("alloc");
        let t = <&T as Bake>::bake(&&**self, ctx);
        quote! {
            alloc::borrow::Cow::Borrowed(#t)
        }
    }
}

impl<T> BakeSize for alloc::borrow::Cow<'_, T>
where
    T: ?Sized + ToOwned,
    for<'a> &'a T: BakeSize,
{
    fn borrows_size(&self) -> usize {
        (&**self).borrows_size()
    }
}

#[test]
fn cow() {
    test_bake!(
        alloc::borrow::Cow<'static, str>,
        const,
        alloc::borrow::Cow::Borrowed("hi"),
        alloc
    );
    assert_eq!(
        BakeSize::borrows_size(&alloc::borrow::Cow::Borrowed("hi")),
        2
    );
    assert_eq!(
        Bake::bake(
            &alloc::borrow::Cow::<'static, str>::Borrowed("hi"),
            &Default::default(),
        )
        .to_string(),
        Bake::bake(
            &alloc::borrow::Cow::<'static, str>::Owned("hi".to_owned()),
            &Default::default(),
        )
        .to_string(),
    );
    assert_eq!(
        BakeSize::borrows_size(&alloc::borrow::Cow::<'static, str>::Owned("hi".to_owned())),
        2
    );
}

impl<T> Bake for Vec<T>
where
    T: Bake,
{
    fn bake(&self, ctx: &CrateEnv) -> TokenStream {
        ctx.insert("alloc");
        let data = self.iter().map(|d| d.bake(ctx));
        quote! {
            alloc::vec![#(#data,)*]
        }
    }
}

impl<T> BakeSize for Vec<T>
where
    T: BakeSize,
{
    fn borrows_size(&self) -> usize {
        self.iter().map(BakeSize::borrows_size).sum()
    }
}

#[test]
fn vec() {
    test_bake!(Vec<u8>, alloc::vec![1u8, 2u8], alloc);
    let v: Vec<&str> = vec!["hello", "world"];
    assert_eq!(BakeSize::borrows_size(&v), 10);
    let empty: Vec<&str> = vec![];
    assert_eq!(BakeSize::borrows_size(&empty), 0);
}

impl<T> Bake for alloc::collections::BTreeSet<T>
where
    T: Bake,
{
    fn bake(&self, ctx: &CrateEnv) -> TokenStream {
        ctx.insert("alloc");
        let data = self.iter().map(|d| d.bake(ctx));
        quote! {
            alloc::collections::BTreeSet::from([#(#data),*])
        }
    }
}

impl<T> BakeSize for alloc::collections::BTreeSet<T>
where
    T: BakeSize,
{
    fn borrows_size(&self) -> usize {
        self.iter().map(BakeSize::borrows_size).sum()
    }
}

#[test]
fn btree_set() {
    test_bake!(
        alloc::collections::BTreeSet<u8>,
        alloc::collections::BTreeSet::from([1u8, 2u8]),
        alloc
    );
    use alloc::collections::BTreeSet;
    let mut set = BTreeSet::new();
    set.insert("hello");
    set.insert("world");
    assert_eq!(BakeSize::borrows_size(&set), 10);
}

impl<K, V> Bake for alloc::collections::BTreeMap<K, V>
where
    K: Bake,
    V: Bake,
{
    fn bake(&self, ctx: &CrateEnv) -> TokenStream {
        ctx.insert("alloc");
        let data = self.iter().map(|(k, v)| {
            let k = k.bake(ctx);
            let v = v.bake(ctx);
            quote!((#k, #v))
        });
        quote! {
            alloc::collections::BTreeMap::from([#(#data),*])
        }
    }
}

impl<K, V> BakeSize for alloc::collections::BTreeMap<K, V>
where
    K: BakeSize,
    V: BakeSize,
{
    fn borrows_size(&self) -> usize {
        self.iter()
            .map(|(k, v)| k.borrows_size() + v.borrows_size())
            .sum()
    }
}

#[test]
fn btree_map() {
    test_bake!(
        alloc::collections::BTreeMap<u8, u8>,
        alloc::collections::BTreeMap::from([(1u8, 2u8), (2u8, 4u8)]),
        alloc
    );
    use alloc::collections::BTreeMap;
    let mut map = BTreeMap::new();
    map.insert("hello", "world");
    map.insert("foo", "bar");
    assert_eq!(BakeSize::borrows_size(&map), 16);
}

impl<T> Bake for HashSet<T>
where
    T: Bake,
{
    fn bake(&self, ctx: &CrateEnv) -> TokenStream {
        ctx.insert("std");
        let mut data = self.iter().map(|d| d.bake(ctx)).collect::<Vec<_>>();
        data.sort_unstable_by_key(|data| data.to_string());
        quote! {
            std::collections::HashSet::from([#(#data),*])
        }
    }
}

impl<T> BakeSize for HashSet<T>
where
    T: BakeSize,
{
    fn borrows_size(&self) -> usize {
        self.iter().map(BakeSize::borrows_size).sum()
    }
}

#[test]
fn hash_set() {
    test_bake!(
        HashSet<u8>,
        std::collections::HashSet::from([1u8, 2u8]),
        std
    );
    let mut set = HashSet::new();
    set.insert("abc");
    set.insert("def");
    assert_eq!(BakeSize::borrows_size(&set), 6);
}

impl<K, V> Bake for std::collections::HashMap<K, V>
where
    K: Bake,
    V: Bake,
{
    fn bake(&self, ctx: &CrateEnv) -> TokenStream {
        ctx.insert("std");
        let mut data = self
            .iter()
            .map(|(k, v)| {
                let k = k.bake(ctx);
                let v = v.bake(ctx);
                quote!((#k, #v))
            })
            .collect::<Vec<_>>();
        data.sort_unstable_by_key(|data| data.to_string());
        quote! {
            std::collections::HashMap::from([#(#data),*])
        }
    }
}

impl<K, V> BakeSize for std::collections::HashMap<K, V>
where
    K: BakeSize,
    V: BakeSize,
{
    fn borrows_size(&self) -> usize {
        self.iter()
            .map(|(k, v)| k.borrows_size() + v.borrows_size())
            .sum()
    }
}

#[test]
fn hash_map() {
    test_bake!(
        std::collections::HashMap<u8, u8>,
        std::collections::HashMap::from([(1u8, 2u8), (2u8, 4u8)]),
        std
    );
    let mut map = std::collections::HashMap::new();
    map.insert("x", "yz");
    map.insert("abc", "d");
    assert_eq!(BakeSize::borrows_size(&map), 7);
    let mut map2 = std::collections::HashMap::new();
    map2.insert("a", "bc");
    map2.insert("de", "f");
    assert_eq!(BakeSize::borrows_size(&map2), 6);
}

impl Bake for String {
    fn bake(&self, _: &CrateEnv) -> TokenStream {
        quote! {
            #self.to_owned()
        }
    }
}

impl BakeSize for String {
    fn borrows_size(&self) -> usize {
        self.len()
    }
}

#[test]
fn string() {
    test_bake!(String, "hello".to_owned());
    assert_eq!(BakeSize::borrows_size(&"hello".to_owned()), 5);
}

macro_rules! smart_pointer_sized {
    ($type:ty, $constructor:path) => {
        impl<T: Sized + Bake> Bake for $type {
            fn bake(&self, ctx: &CrateEnv) -> TokenStream {
                ctx.insert("alloc");
                let data = std::ops::Deref::deref(self).bake(ctx);
                quote! { $constructor(#data) }
            }
        }
        impl<T: Sized + BakeSize> BakeSize for $type {
            fn borrows_size(&self) -> usize {
                (**self).borrows_size()
            }
        }
    };
}

macro_rules! smart_pointer_unsized {
    ($type:ty, $constructor:expr) => {
        impl<T: Bake> Bake for $type {
            fn bake(&self, ctx: &CrateEnv) -> TokenStream {
                ctx.insert("alloc");
                let vec: Vec<_> = self.iter().collect();
                let baked = vec.bake(ctx);
                quote! { $constructor(#baked) }
            }
        }
        impl<T: BakeSize> BakeSize for $type {
            fn borrows_size(&self) -> usize {
                self.iter().map(BakeSize::borrows_size).sum()
            }
        }
    };
}

smart_pointer_sized!(Box<T>, Box::new);
smart_pointer_unsized!(Box<[T]>, |v: Vec<T>| v.into_boxed_slice());

smart_pointer_sized!(alloc::rc::Rc<T>, alloc::rc::Rc::new);
smart_pointer_unsized!(alloc::rc::Rc<[T]>, |v: Vec<T>| alloc::rc::Rc::from(
    v.into_boxed_slice()
));

smart_pointer_sized!(alloc::sync::Arc<T>, alloc::sync::Arc::new);
smart_pointer_unsized!(alloc::sync::Arc<[T]>, |v: Vec<T>| alloc::sync::Arc::from(
    v.into_boxed_slice()
));

#[test]
fn smart_pointers() {
    test_bake!(Box<char>, Box::new('a'), alloc);
    test_bake!(alloc::rc::Rc<char>, alloc::rc::Rc::new('b'), alloc);
    test_bake!(alloc::sync::Arc<char>, alloc::sync::Arc::new('c'), alloc);

    let boxed: Box<[&str]> = vec!["hello", "world"].into_boxed_slice();
    assert_eq!(BakeSize::borrows_size(&boxed), 10);
    let rced: alloc::rc::Rc<[&str]> = vec!["foo", "bar"].into();
    assert_eq!(BakeSize::borrows_size(&rced), 6);
    let arced: alloc::sync::Arc<[&str]> = vec!["x", "y", "z"].into();
    assert_eq!(BakeSize::borrows_size(&arced), 3);
}
