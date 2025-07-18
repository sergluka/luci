use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;

use bimap::BiHashMap;
use elfo::Addr;
use serde_json::Value;
use tracing::info;

use crate::{bindings, names::ActorName};

#[derive(Debug, thiserror::Error)]
pub enum BindError {
    #[error("unbound value: {}", _0)]
    UnboundValue(String),
}

/// Stores bindings:
/// - luci variables bound to [values](Value);
/// - actor names bound to [addresses](Addr).
#[derive(Debug, Default)]
pub(crate) struct Scope {
    values: HashMap<String, Value>,
    actors: BiHashMap<ActorName, Addr>,
}

/// A transaction on a [Scope].
///
/// Bindings to variables and addresses can be added to the transaction.
/// Then a transaction can be either commited (applied to the [Scope]) or
/// dropped.
#[derive(Debug)]
pub(crate) struct Txn<'a> {
    values_committed: &'a mut HashMap<String, Value>,
    values_added: HashMap<String, Value>,

    actors_committed: &'a mut BiHashMap<ActorName, Addr>,
    actors_added: BiHashMap<ActorName, Addr>,
}

impl Scope {
    /// Creates a [Txn] on the current state of the [Scope].
    pub(crate) fn txn(&mut self) -> Txn {
        Txn {
            values_committed: &mut self.values,
            values_added: Default::default(),

            actors_committed: &mut self.actors,
            actors_added: Default::default(),
        }
    }

    /// Returns bound [Addr] for the specified `name` if there is one.
    /// Otherwise returns `None`.
    pub(crate) fn address_of(&self, name: &ActorName) -> Option<Addr> {
        self.actors.get_by_left(name).copied()
    }

    /// Returns bound [Value] for the specified `key` if there is one.
    /// Otherwise returns `None`.
    fn value_of(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }
}

impl<'a> Txn<'a> {
    /// Binds `key` to `value` and stores in the transaction.
    pub(crate) fn bind_value(&mut self, key: &str, value: &Value) -> bool {
        if let Some(defined_in_state) = self.values_committed.get(key) {
            defined_in_state == value
        } else {
            match self.values_added.entry(key.to_owned()) {
                Occupied(o) => o.get() == value,
                Vacant(v) => {
                    v.insert(value.to_owned());
                    true
                }
            }
        }
    }

    /// Binds `name` to `addr` and stores in the transaction.
    pub(crate) fn bind_actor(&mut self, name: &ActorName, addr: Addr) -> bool {
        if let Some(existing_name) = {
            let old_opt = self.actors_committed.get_by_right(&addr);
            let new_opt = self.actors_added.get_by_right(&addr);

            old_opt.or(new_opt)
        } {
            return existing_name == name;
        }
        if let Some(existing_addr) = {
            let old_opt = self.actors_committed.get_by_left(name).copied();
            let new_opt = self.actors_added.get_by_left(name).copied();

            old_opt.or(new_opt)
        } {
            assert!(existing_addr != addr);
            return false;
        }

        self.actors_added
            .insert_no_overwrite(name.clone(), addr)
            .expect("none of the sides resolved before!");
        true
    }

    /// Commits transaction to the [Scope].
    pub(crate) fn commit(self) {
        self.values_committed.extend(
            self.values_added
                .into_iter()
                .inspect(|(k, v)| info!("SET VALUE {:?} <- {:?}", k, v)),
        );
        self.actors_committed.extend(
            self.actors_added
                .into_iter()
                .inspect(|(k, v)| info!("SET ACTOR {:?} <- {:?}", k, v)),
        );
    }
}

/// Binds luci variables from `value` according to `pattern` and adds the result
/// to `bindings`.
pub(crate) fn bind_to_pattern(value: Value, pattern: &Value, bindings: &mut Txn) -> bool {
    match (value, pattern) {
        (_, Value::String(wildcard)) if wildcard == "$_" => true,

        (value, Value::String(var_name)) if var_name.starts_with('$') => {
            bindings.bind_value(&var_name, &value)
        }

        (Value::Null, Value::Null) => true,
        (Value::Bool(v), Value::Bool(p)) => v == *p,
        (Value::String(v), Value::String(p)) => v == *p,
        (Value::Number(v), Value::Number(p)) => v == *p,
        (Value::Array(values), Value::Array(patterns)) => {
            values.len() == patterns.len()
                && values
                    .into_iter()
                    .zip(patterns)
                    .all(|(v, p)| bind_to_pattern(v, p, bindings))
        }

        (Value::Object(mut v), Value::Object(p)) => p.iter().all(|(pk, pv)| {
            v.remove(pk)
                .is_some_and(|vv| bind_to_pattern(vv, pv, bindings))
        }),

        (_, _) => false,
    }
}

/// Renders luci variables in `template` with values from `bindings`.
///
/// Returns:
/// - The resulting [Value] after template render on success;
/// - [BindError] on error.
pub(crate) fn render(template: Value, bindings: &bindings::Scope) -> Result<Value, BindError> {
    match template {
        Value::String(wildcard) if wildcard == "$_" => Err(BindError::UnboundValue(wildcard)),
        Value::String(var_name) if var_name.starts_with('$') => bindings
            .value_of(&var_name)
            .cloned()
            .ok_or_else(|| BindError::UnboundValue(var_name)),
        Value::Array(items) => Ok(Value::Array(
            items
                .into_iter()
                .map(|item| render(item, bindings))
                .collect::<Result<_, _>>()?,
        )),
        Value::Object(kv) => Ok(Value::Object(
            kv.into_iter()
                .map(|(k, v)| render(v, bindings).map(move |v| (k, v)))
                .collect::<Result<_, _>>()?,
        )),
        as_is => Ok(as_is),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    impl Scope {
        pub(crate) fn new() -> Self {
            Default::default()
        }
    }

    #[test]
    fn test_01() {
        let mut scope = Scope::new();
        assert!(scope.value_of("a").is_none());
        assert!(scope.value_of("b").is_none());

        {
            let mut txn = scope.txn();

            assert!(txn.bind_value("a", &json!("a")));
            assert!(txn.bind_value("a", &json!("a")));
            assert!(!txn.bind_value("a", &json!("b")));
        }

        assert!(scope.value_of("a").is_none());
        assert!(scope.value_of("b").is_none());

        {
            let mut txn = scope.txn();

            assert!(txn.bind_value("a", &json!("a")));
            assert!(txn.bind_value("a", &json!("a")));
            assert!(!txn.bind_value("a", &json!("b")));

            txn.commit();
        }

        assert_eq!(scope.value_of("a").cloned(), Some(json!("a")));
        assert!(scope.value_of("b").is_none());
    }
}
