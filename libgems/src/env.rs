#![allow(private_bounds)]

use std::{collections::HashMap, marker::PhantomData};

use crate::theme;

trait AllEnvValues: Sized {
    fn from_env_value(v: EnvValue) -> Option<Self>;
    fn into_env_value(self) -> EnvValue;
}

impl AllEnvValues for bool {
    fn from_env_value(v: EnvValue) -> Option<Self> {
        match v {
            EnvValue::Bool(b) => Some(b),
            _ => None,
        }
    }
    fn into_env_value(self) -> EnvValue {
        EnvValue::Bool(self)
    }
}

impl AllEnvValues for super::Color {
    fn from_env_value(v: EnvValue) -> Option<Self> {
        match v {
            EnvValue::Color(c) => Some(c),
            _ => None,
        }
    }
    fn into_env_value(self) -> EnvValue {
        EnvValue::Color(self)
    }
}

#[derive(Debug, Clone, Copy)]
enum EnvValue {
    Color(super::Color),
    Bool(bool),
}

#[derive(Debug, Clone, Copy)]
/// Represents an Error trying to reterive an Env value.
pub enum EnvError {
    NoValue,
    UnexpectedValue,
}

/// Represents a key into an [`AppEnv`].
#[derive(Debug, Clone, Copy)]
pub struct EnvKey<Result: AllEnvValues> {
    key: &'static str,
    holds: PhantomData<Result>,
}

impl<R: AllEnvValues> EnvKey<R> {
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            holds: PhantomData,
        }
    }
}

/// The theme and UI Environment of the App.
pub struct AppEnv {
    inner: HashMap<String, EnvValue>,
}

impl AppEnv {
    /// Attempts to get the given `key` from the env.
    pub fn try_get<R: AllEnvValues>(&self, key: EnvKey<R>) -> Result<R, EnvError> {
        let env_val = self.inner.get(key.key).ok_or(EnvError::NoValue)?;
        R::from_env_value(*env_val).ok_or(EnvError::UnexpectedValue)
    }

    /// Same as [`Self::try_get`] but panics on failure.
    pub fn get<R: AllEnvValues>(&self, key: EnvKey<R>) -> R {
        self.try_get(key).expect("Failed to reterive Env Key")
    }

    /// Sets the given key to the given value.
    pub fn set_key<R: AllEnvValues>(&mut self, key: EnvKey<R>, value: R) -> &mut Self {
        self.inner.insert(key.key.into(), value.into_env_value());
        self
    }
}

impl Default for AppEnv {
    fn default() -> Self {
        let mut this = AppEnv {
            inner: HashMap::new(),
        };

        theme::default_theme(&mut this);
        this
    }
}
