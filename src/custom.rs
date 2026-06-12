use super::*;

#[cfg(feature = "uuid")]
impl Bake for uuid::Uuid {
    fn bake(&self, ctx: &CrateEnv) -> TokenStream {
        let bytes = self.as_bytes();
        let baked = bytes.bake(ctx);
        quote! { uuid::Uuid::from_bytes(#baked) }
    }
}

#[cfg(feature = "uuid")]
impl BakeSize for uuid::Uuid {
    fn borrows_size(&self) -> usize {
        16
    }
}
