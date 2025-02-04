// Copyright 2019 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use crate::error::Error;
use crate::jsonrpc;

use alloc::string::String;
use core::fmt;

/// Access to the parameters of a request.
#[derive(Copy, Debug, Clone)]
pub struct Params<'a> {
	/// Raw parameters of the request.
	params: &'a jsonrpc::Params,
}

/// Key referring to a potential parameter of a request.
pub enum ParamKey<'a> {
	/// String key. Only valid when the parameters list is a map.
	String(&'a str),
	/// Integer key. Only valid when the parameters list is an array.
	Index(usize),
}

impl<'a> Params<'a> {
	/// Wraps around a `&jsonrpc::Params` and provides utility functions for the user.
	pub(crate) fn from(params: &'a jsonrpc::Params) -> Params<'a> {
		Params { params }
	}

	/// Returns a parameter of the request by name and decodes it.
	///
	/// Returns an error if the parameter doesn't exist or is of the wrong type.
	pub fn get<'k, T>(self, param: impl Into<ParamKey<'k>>) -> Result<T, Error>
	where
		T: serde::de::DeserializeOwned,
	{
		let val = self.get_raw(param).ok_or_else(|| Error::Custom("No such param".into()))?;
		serde_json::from_value(val.clone()).map_err(Error::ParseError)
	}

	/// Returns a parameter of the request by name.
	pub fn get_raw<'k>(self, param: impl Into<ParamKey<'k>>) -> Option<&'a jsonrpc::JsonValue> {
		match (self.params, param.into()) {
			(jsonrpc::Params::None, _) => None,
			(jsonrpc::Params::Map(map), ParamKey::String(key)) => map.get(key),
			(jsonrpc::Params::Map(_), ParamKey::Index(_)) => None,
			(jsonrpc::Params::Array(_), ParamKey::String(_)) => None,
			(jsonrpc::Params::Array(array), ParamKey::Index(index)) => {
				if index < array.len() {
					Some(&array[index])
				} else {
					None
				}
			}
		}
	}
}

impl<'a> IntoIterator for Params<'a> {
	type Item = Entry<'a>;
	type IntoIter = Iter<'a>;

	fn into_iter(self) -> Self::IntoIter {
		Iter(match self.params {
			jsonrpc::Params::None => IterInner::Empty,
			jsonrpc::Params::Array(arr) => IterInner::Array(arr.iter()),
			jsonrpc::Params::Map(map) => IterInner::Map(map.iter()),
		})
	}
}

impl<'a> AsRef<jsonrpc::Params> for Params<'a> {
	fn as_ref(&self) -> &jsonrpc::Params {
		self.params
	}
}

impl<'a> From<Params<'a>> for &'a jsonrpc::Params {
	fn from(params: Params<'a>) -> &'a jsonrpc::Params {
		params.params
	}
}

impl<'a> From<&'a str> for ParamKey<'a> {
	fn from(s: &'a str) -> Self {
		ParamKey::String(s)
	}
}

impl<'a> From<&'a String> for ParamKey<'a> {
	fn from(s: &'a String) -> Self {
		ParamKey::String(&s[..])
	}
}

impl<'a> From<usize> for ParamKey<'a> {
	fn from(i: usize) -> Self {
		ParamKey::Index(i)
	}
}

impl<'a> fmt::Debug for ParamKey<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			ParamKey::String(s) => fmt::Debug::fmt(s, f),
			ParamKey::Index(s) => fmt::Debug::fmt(s, f),
		}
	}
}

/// Iterator to all the parameters of a request.
pub struct Iter<'a>(IterInner<'a>);

enum IterInner<'a> {
	Empty,
	Map(serde_json::map::Iter<'a>),
	Array(std::slice::Iter<'a, serde_json::Value>),
}

#[derive(Debug)]
pub enum Entry<'a> {
	Value(&'a jsonrpc::JsonValue),
	KeyValue(ParamKey<'a>, &'a jsonrpc::JsonValue),
}

impl<'a> Iterator for Iter<'a> {
	type Item = Entry<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		match &mut self.0 {
			IterInner::Empty => None,
			IterInner::Map(iter) => iter.next().map(|(k, v)| Entry::KeyValue(ParamKey::String(&k[..]), v)),
			IterInner::Array(iter) => iter.next().map(|v| Entry::Value(v)),
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		match &self.0 {
			IterInner::Empty => (0, Some(0)),
			IterInner::Map(iter) => iter.size_hint(),
			IterInner::Array(iter) => iter.size_hint(),
		}
	}
}

impl<'a> ExactSizeIterator for Iter<'a> {}

impl<'a> fmt::Debug for Iter<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("ParamsIter").finish()
	}
}
