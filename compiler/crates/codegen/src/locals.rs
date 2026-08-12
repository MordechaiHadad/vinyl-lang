use std::marker::PhantomData;

use vinyl_typecheck::hir::Type;

/// Backend operation used by the shared local-variable builder.
pub trait FunctionBackend {
    /// Backend value handle.
    type Value: Copy;
    /// Backend local-storage handle.
    type Storage;
    /// Backend diagnostic error.
    type Error: miette::Diagnostic + Send + Sync + 'static;

    /// Declares and initializes a local using backend-specific storage policy.
    fn declare_local(
        &mut self,
        name: &str,
        type_: &Type,
        value: Self::Value,
        mutable: bool,
        address_taken: bool,
    ) -> Result<Self::Storage, Self::Error>;

    /// Loads a local value.
    fn load_local(&mut self, storage: &Self::Storage) -> Result<Self::Value, Self::Error>;

    /// Stores a new value into a local.
    fn store_local(
        &mut self,
        storage: &Self::Storage,
        value: Self::Value,
    ) -> Result<(), Self::Error>;

    /// Creates an error for an incomplete declaration builder.
    fn invalid_local(message: &'static str) -> Self::Error;
}

/// Semantic builder for a local declaration.
pub struct LocalBuilder<'a, B: FunctionBackend> {
    backend: &'a mut B,
    name: String,
    type_: Option<Type>,
    value: Option<B::Value>,
    mutable: bool,
    address_taken: bool,
    marker: PhantomData<B>,
}

impl<'a, B: FunctionBackend> LocalBuilder<'a, B> {
    /// Starts a declaration for `name`.
    pub fn new(backend: &'a mut B, name: impl Into<String>) -> Self {
        Self {
            backend,
            name: name.into(),
            type_: None,
            value: None,
            mutable: false,
            address_taken: false,
            marker: PhantomData,
        }
    }

    /// Supplies the declared Vinyl type.
    pub fn typed(mut self, type_: Type) -> Self {
        self.type_ = Some(type_);
        self
    }

    /// Supplies the initializer value.
    pub fn initialized(mut self, value: B::Value) -> Self {
        self.value = Some(value);
        self
    }

    /// Marks the local mutable.
    pub fn mutable(mut self) -> Self {
        self.mutable = true;
        self
    }

    /// Marks the local mutable when `condition` is true.
    pub fn mutable_if(self, condition: bool) -> Self {
        if condition { self.mutable() } else { self }
    }

    /// Marks the local as address-taken.
    pub fn address_taken(mut self) -> Self {
        self.address_taken = true;
        self
    }

    /// Marks the local address-taken when `condition` is true.
    pub fn address_taken_if(self, condition: bool) -> Self {
        if condition {
            self.address_taken()
        } else {
            self
        }
    }

    /// Builds the backend storage handle.
    pub fn build(self) -> Result<B::Storage, B::Error> {
        let type_ = self
            .type_
            .as_ref()
            .ok_or_else(|| B::invalid_local("local declaration is missing a type"))?;
        let value = self
            .value
            .ok_or_else(|| B::invalid_local("local declaration is missing an initializer"))?;
        self.backend
            .declare_local(&self.name, type_, value, self.mutable, self.address_taken)
    }
}
